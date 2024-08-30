use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use qdrant_client::{qdrant::Distance, Qdrant};
use std::sync::Arc;

use chrono::NaiveDate;
use cyclicism::{
    get_date, get_json_path,
    mydrant::{break_article_for_mydrant, BedSource, Collection, DetailedEmbedding},
    nyt::ScrapedJson,
};
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    task::JoinSet,
};

async fn error_thread(mut rx: Receiver<(String, String)>) {
    while let Some((file, msg)) = rx.recv().await {
        println!("\x1b[33m{}\x1b[0m\n\x1b[31m{}\x1b[0m", file, msg);
    }
}

async fn worker_thread(
    collection: Arc<Collection>,
    loaded_model: Arc<TextEmbedding>,
    data: Arc<Mutex<Vec<NaiveDate>>>,
    tx: Sender<(String, String)>,
) {
    loop {
        let date = {
            let mut lock = data.lock().await;
            let Some(date) = lock.pop() else {
                break;
            };
            println!("{} months left!", lock.len());
            date
        };
        let scraped = match ScrapedJson::from_date(date) {
            Ok(val) => val,
            Err(e) => {
                tx.send((format!("{:?}", get_json_path(date)), format!("{:?}", e)))
                    .await
                    .ok();
                break;
            }
        };
        let mut articles = scraped.response.docs;
        while !articles.is_empty() {
            let chunk = articles
                .drain(0..CHUNK_SIZE.min(articles.len()))
                .map(|article| break_article_for_mydrant(article, BED_SOURCE))
                .filter_map(|details| details);
            let mut documents = vec![];
            let mut broad_details = vec![];
            for (uri, text, info) in chunk {
                documents.push(text);
                broad_details.push((uri, info));
            }
            let model_arc = loaded_model.clone();
            let bed_hand = tokio::task::spawn_blocking(move || model_arc.embed(documents, None));
            let beds = match bed_hand.await {
                Ok(Ok(beds)) => beds,
                Ok(Err(e)) => {
                    tx.send((format!("{:?}", get_json_path(date)), format!("{:?}", e)))
                        .await
                        .ok();
                    break;
                }
                Err(e) => {
                    tx.send((format!("{:?}", get_json_path(date)), format!("{:?}", e)))
                        .await
                        .ok();
                    break;
                }
            };
            let data = beds
                .into_iter()
                .zip(broad_details.into_iter())
                .map(|(bed, (uuid, info))| DetailedEmbedding { uuid, bed, info })
                .collect::<Vec<_>>();
            if let Err(e) = collection.upsert(data).await {
                tx.send((format!("{:?}", get_json_path(date)), format!("{:?}", e)))
                    .await
                    .ok();
                break;
            };
        }
    }
}

const NUM_WORKERS: u32 = 4;
const CHUNK_SIZE: usize = 64;
const BED_SOURCE: BedSource = BedSource::HeadlineMain;
const BED_DIM: u64 = 1024;
const BED_MODEL: EmbeddingModel = EmbeddingModel::GTELargeENV15Q;
const DISTANCE: Distance = Distance::Cosine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let qdrant = Qdrant::from_url("http://localhost:6334").build()?;
    let collection = Arc::new(Collection::new(
        BED_SOURCE, BED_DIM, BED_MODEL, DISTANCE, qdrant,
    ));
    collection.ensure_created().await?;
    let loaded_model = Arc::new(TextEmbedding::try_new(
        InitOptions::new(BED_MODEL).with_show_download_progress(true),
    )?);

    let mut raw_data = vec![];
    for year in cyclicism::START_YEAR..=cyclicism::END_YEAR {
        let mut month = 1;
        while month <= 12 {
            let date = get_date(year, month);
            raw_data.push(date);
            month += 1;
        }
    }
    let data = Arc::new(Mutex::new(raw_data));
    let mut set = JoinSet::new();
    let (tx, rx) = channel(64);
    set.spawn(error_thread(rx));
    for _ in 0..NUM_WORKERS {
        set.spawn(worker_thread(
            collection.clone(),
            loaded_model.clone(),
            data.clone(),
            tx.clone(),
        ));
    }
    drop(tx); // If we don't drop this the error thread never dies...
    while let Some(_) = set.join_next().await {}
    Ok(())
}
