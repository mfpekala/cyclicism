use std::sync::Arc;

use chrono::NaiveDate;
use cyclicism::{
    get_date, get_json_path,
    nyt::ScrapedJson,
    pg::{apply_migrations, get_pool},
};
use sqlx::PgPool;
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    task::JoinSet,
};

const NUM_WORKERS: u32 = 6;

async fn error_thread(mut rx: Receiver<(String, String)>) {
    while let Some((file, msg)) = rx.recv().await {
        println!("\x1b[33m{}\x1b[0m\n\x1b[31m{}\x1b[0m", file, msg);
    }
}

async fn worker_thread(
    conn: Arc<PgPool>,
    data: Arc<Mutex<Vec<NaiveDate>>>,
    tx: Sender<(String, String)>,
) {
    loop {
        let date = {
            let mut lock = data.lock().await;
            let Some(date) = lock.pop() else {
                break;
            };
            println!("{} left!", lock.len());
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
        for article in &scraped.response.docs {
            if let Err(e) = article.upsert(&conn).await {
                tx.send((format!("{:?}", get_json_path(date)), format!("{:?}", e)))
                    .await
                    .ok();
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool: Arc<PgPool> = Arc::new(get_pool(NUM_WORKERS + 2).await?.into());
    apply_migrations(&pool).await?;

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
        set.spawn(worker_thread(pool.clone(), data.clone(), tx.clone()));
    }
    drop(tx); // If we don't drop this the error thread never dies...
    while let Some(_) = set.join_next().await {}
    Ok(())
}
