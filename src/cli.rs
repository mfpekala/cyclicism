use cyclicism::{
    mydrant::{BedSource, Collection},
    nyt::FrontendArticle,
    pg::get_pool,
};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use qdrant_client::{qdrant::Distance, Qdrant};
use sqlx::PgPool;

const BED_SOURCE: BedSource = BedSource::HeadlineMain;
const BED_DIM: u64 = 1024;
const BED_MODEL: EmbeddingModel = EmbeddingModel::GTELargeENV15Q;
const DISTANCE: Distance = Distance::Cosine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let qdrant = Qdrant::from_url("http://localhost:6334").build()?;
    let collection = Collection::new(BED_SOURCE, BED_DIM, BED_MODEL, DISTANCE, qdrant);
    let pool: PgPool = get_pool(6).await?.into();
    let loaded_model =
        TextEmbedding::try_new(InitOptions::new(BED_MODEL).with_show_download_progress(true))?;
    loop {
        let mut raw_input = String::new();
        println!("Enter a headline: ");
        std::io::stdin().read_line(&mut raw_input)?;
        let input = raw_input.trim();
        if input == "quit" {
            break;
        }
        let bed = loaded_model
            .embed(vec![input], None)?
            .into_iter()
            .next()
            .unwrap();
        let infos = collection.top_k(bed, 5).await?;
        let mut articles = vec![];
        for info in infos {
            let article = FrontendArticle::from_uri(&info.uri, &pool).await?;
            articles.push(article);
        }
        for (ix, article) in articles.into_iter().enumerate() {
            println!("Result {ix}");
            println!("Headline: {}", article.headline_main);
            println!("Date: {}/{}/{}", article.month, article.day, article.year);
            println!("Snippet: {}", article.snippet);
            println!("URL: {}", article.web_url);
            println!("\n");
        }
    }

    Ok(())
}
