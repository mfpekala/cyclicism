use axum::{routing::get, Router};
use cyclicism::{
    nyt::FrontendArticle,
    pg::{apply_migrations, get_pg_pool},
};
use tracing::info;

mod combos_on_date;
use combos_on_date::get_combos_on_date;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Database
    let pg_pool = get_pg_pool(6).await?;
    apply_migrations(&pg_pool).await?;

    // Routing
    let app = Router::new()
        .route("/combos_on_date", get(get_combos_on_date))
        .with_state(pg_pool);

    // Run it
    info!(
        r#"Welcome to the cyclicism CRUNCH api...
     /\
   .'  `.
 .'      `.
<          >
 `.      .'
   `.  .'
     \/"#
    );
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// NOTE: Rn these types are just wrappers, nice to have if we want specialized data on either
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ContemporaryArticle {
    article: FrontendArticle,
}

/// NOTE: Rn these types are just wrappers, nice to have if we want specialized data on either
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PastArticle {
    article: FrontendArticle,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Combo {
    contemporary: ContemporaryArticle,
    past: Vec<PastArticle>,
}
