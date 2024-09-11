use cyclicism::{
    mydrant::{BedSource, Collection},
    nyt::{get_current_homepage, ContemporaryArticle},
    pg::get_pg_pool,
};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use qdrant_client::{qdrant::Distance, Qdrant};
use sqlx::Row;
use sqlx::{Pool, Postgres};
use std::{env, sync::Arc};

/// Given a list of contemporary articles, filter down to only those without combos
async fn filter_new_articles<'a>(
    all_articles: &'a Vec<ContemporaryArticle>,
    pg: &Pool<Postgres>,
) -> anyhow::Result<Vec<&'a ContemporaryArticle>> {
    let mut new_ones = vec![];
    for article in all_articles {
        let Ok(count_row) = sqlx::query(
            r#"
            SELECT COUNT(*)
            FROM combos
            WHERE contemporary_uri = $1"#,
        )
        .bind(&article.uri)
        .fetch_one(pg)
        .await
        else {
            continue;
        };
        let count: i64 = count_row.get(0);
        if count > 0 {
            continue;
        }
        new_ones.push(article);
    }
    Ok(new_ones)
}

const BED_SOURCE: BedSource = BedSource::HeadlineMain;
const BED_DIM: u64 = 1024;
const BED_MODEL: EmbeddingModel = EmbeddingModel::GTELargeENV15Q;
const DISTANCE: Distance = Distance::Cosine;

/// Given all of the current articles, embed and add combos only for those that need it
async fn update_combos(
    current_articles: &Vec<ContemporaryArticle>,
    pg: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let qdrant = Qdrant::from_url("http://localhost:6334").build()?;
    let collection = Arc::new(Collection::new(
        BED_SOURCE, BED_DIM, BED_MODEL, DISTANCE, qdrant,
    ));
    let loaded_model = Arc::new(TextEmbedding::try_new(
        InitOptions::new(BED_MODEL).with_show_download_progress(true),
    )?);
    let unseen = filter_new_articles(&current_articles, pg).await?;
    println!("unseen: {} vs {}", current_articles.len(), unseen.len());
    for article in unseen {
        let bed = loaded_model
            .embed(vec![article.title.clone()], None)?
            .into_iter()
            .next()
            .unwrap();
        let scored_infos = collection.top_k(bed, 10).await?;
        for (info, score) in scored_infos {
            sqlx::query(
                r#"
            INSERT INTO combos (contemporary_uri, past_uri, score)
            VALUES ($1, $2, $3)
            "#,
            )
            .bind(&article.uri)
            .bind(&info.uri)
            .bind(score)
            .execute(pg)
            .await
            .ok();
        }
        article.upsert(pg).await.ok();
    }
    Ok(())
}

/// Truncate and remake the current table
async fn remake_current(
    current_articles: &Vec<ContemporaryArticle>,
    pg: &Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        TRUNCATE TABLE current
        "#,
    )
    .execute(pg)
    .await?;
    for (ix, article) in current_articles.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO current (uri, rank)
            VALUES ($1, $2)
            "#,
        )
        .bind(&article.uri)
        .bind(ix as i32)
        .execute(pg)
        .await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = env::var("NYT_API_KEY").unwrap();
    let pool = get_pg_pool(2).await?;

    let current_articles = get_current_homepage(&api_key).await?;
    update_combos(&current_articles, &pool).await?;
    remake_current(&current_articles, &pool).await?;

    Ok(())
}
