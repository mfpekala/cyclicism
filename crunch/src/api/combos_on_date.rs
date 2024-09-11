use axum::{extract::Query, http::StatusCode, Json};

use crate::Combo;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CombosOnDateReq {
    year: u32,
    month: u32,
    day: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CombosOnDateResp {
    combos: Vec<Combo>,
}

/// Returns the stories from a given date. This includes the actual stories from that day,
/// and the stories in our index that were most similar.
#[tracing::instrument]
pub async fn get_combos_on_date(
    Query(req): Query<CombosOnDateReq>,
) -> Result<Json<CombosOnDateResp>, StatusCode> {
    Ok(Json(CombosOnDateResp { combos: vec![] }))
}

/// Why? Can't have current articles look up to current articles
///
/// So I should make:
/// contemporary_article (should be as similar to scraped_article, but my own date semantics)
/// contemporary_headline...
/// contemporary_multimedia...
///
/// Task which every hour...
/// - Fetches contemporary.
/// - For each new article
///     - Puts it into contemporary tables on pg
///     - Embed it and find most similar
///     - Stores those results in pg combos table
/// - Completely drops and remakes a "current" table of just the uris, basically to not hit NYT rate limits
///
/// There are then two endpoints:
/// - past_date
///     - Will get the contemporary articles published on this day.
///     - Will fetch their combos and show
/// - current
///     - Will read from the "current" table and use that
///     - Will fetch their combos and show
///
/// EVERYTHING BELOW THIS IS STUPID
/// Okay so my brain is currently battling with this fact:
/// How do I actually display results per day? While also doing current? Hmmm.
///
/// Here's what I think I'll do.
///
/// 1. I need to see if I can scrape current articles into `scraped_article (+ scraped_headline + scraped_multimedia)`
///
/// YES:
/// 2. Make a task which every hour, scrapes, puts them into into scraped_article (+ ...).
/// 3. ^This script should TRY to embed and put into quadrant, but NOT fail if it fails
/// 4. Make another task which (at some cadence, probably daily) looks for articles that are NOT in qdrant, and embeds them
///
struct BrainGoBrr;
