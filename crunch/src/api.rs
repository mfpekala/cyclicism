use axum::{
    extract::{Json, Query},
    routing::get,
    Router,
};
use cyclicism::nyt::FrontendArticle;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Routing
    let app = Router::new().route("/combos_on_date", get(combos_on_date));

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CombosOnDateReq {
    year: u32,
    month: u32,
    day: u32,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CombosOnDateResp {
    combos: Vec<Combo>,
}

/// Returns the stories from a given date. This includes the actual stories from that day,
/// and the stories in our index that were most similar.
#[tracing::instrument]
async fn combos_on_date(Query(req): Query<CombosOnDateReq>) -> Json<CombosOnDateResp> {
    CombosOnDateResp { combos: vec![] }.into()
}
