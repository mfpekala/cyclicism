use chrono::{DateTime, Datelike, FixedOffset, NaiveDate};
use regex::Regex;
use uuid::Uuid;

use crate::get_json_path;

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedMeta {
    pub hits: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedMultimediaLegacy {
    pub xlarge: Option<String>,
    pub xlargewidth: Option<u32>,
    pub xlargeheight: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedMultimedia {
    pub rank: u32,
    pub subtype: String,
    pub caption: Option<String>,
    pub credit: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
    pub height: u32,
    pub width: u32,
    pub legacy: ScrapedMultimediaLegacy,
    pub crop_name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedHeadline {
    pub main: String,
    pub kicker: Option<String>,
    pub content_kicker: Option<String>,
    pub print_headline: String,
    pub name: Option<String>,
    pub seo: Option<String>,
    pub sub: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedKeyword {
    pub name: String,
    pub value: String,
    pub rank: u32,
    pub major: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedPerson {
    pub firstname: String,
    pub middlename: Option<String>,
    pub lastname: String,
    pub qualifier: Option<String>,
    pub title: Option<String>,
    pub role: String,
    pub organization: String,
    pub rank: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedByline {
    pub original: String,
    pub person: Vec<ScrapedPerson>,
    pub organization: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedArticle {
    pub web_url: String,
    pub snippet: String,
    pub print_page: Option<String>,
    pub print_section: Option<String>,
    pub source: String,
    pub multimedia: Vec<ScrapedMultimedia>,
    pub headline: ScrapedHeadline,
    pub keywords: Vec<ScrapedKeyword>,
    pub pub_date: String,
    pub document_type: String,
    pub news_desk: String,
    pub section_name: String,
    pub byline: ScrapedByline,
    pub type_of_material: String,
    pub uri: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedResponse {
    pub meta: ScrapedMeta,
    pub docs: Vec<ScrapedArticle>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScrapedJson {
    pub copyright: String,
    pub response: ScrapedResponse,
}
impl ScrapedJson {
    pub fn from_date(date: NaiveDate) -> anyhow::Result<Self> {
        let alleged_path = get_json_path(date);
        let contents = std::fs::read_to_string(alleged_path)?;
        let scraped_json: ScrapedJson = serde_json::from_str(&contents)?;
        Ok(scraped_json)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ContemporaryMultimedia {
    pub url: String,
    pub format: String,
    pub height: u32,
    pub width: u32,
    #[serde(rename = "type")]
    pub type_: String,
    pub subtype: String,
    pub caption: String,
    pub copyright: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ContemporaryArticle {
    pub section: String,
    pub subsection: String,
    pub title: String,
    #[serde(rename = "abstract")]
    pub abstract_: String,
    pub url: String,
    pub uri: String,
    pub byline: String,
    pub item_type: String,
    pub updated_date: String,
    pub created_date: String,
    pub published_date: String,
    pub material_type_facet: String,
    pub kicker: String,
    pub des_facet: Vec<String>,
    pub org_facet: Vec<String>,
    pub per_facet: Vec<String>,
    pub geo_facet: Vec<String>,
    pub multimedia: Option<Vec<ContemporaryMultimedia>>,
    pub short_url: String,
}
impl ContemporaryArticle {
    pub fn get_date_parts(&self) -> anyhow::Result<(u32, u32, u32)> {
        let dt: DateTime<FixedOffset> =
            DateTime::parse_from_str(&self.published_date, "%Y-%m-%dT%H:%M:%S%:z").unwrap();
        Ok((dt.year_ce().1 as u32, dt.month0() + 1, dt.day()))
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ContemporaryJson {
    pub status: String,
    pub copyright: String,
    pub section: String,
    pub last_updated: String,
    pub num_results: u32,
    pub results: Vec<ContemporaryArticle>,
}

/// Fetches the current articles on the homepage as `ContemporaryArticle`s.
pub async fn get_current_homepage(api_key: &str) -> anyhow::Result<Vec<ContemporaryArticle>> {
    let resp = reqwest::get(format!(
        "https://api.nytimes.com/svc/topstories/v2/home.json?api-key={}",
        api_key
    ))
    .await?;
    let text = resp.text().await?;
    let obj = serde_json::from_str::<ContemporaryJson>(&text)?;
    Ok(obj.results)
}

pub fn uri_to_uuid(uri: &str) -> Uuid {
    Uuid::new_v3(&Uuid::NAMESPACE_URL, uri.as_bytes())
}

pub fn parse_pub_date(date_str: &str) -> NaiveDate {
    let dt: DateTime<FixedOffset> =
        DateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%:z").unwrap();
    NaiveDate::from_ymd_opt(dt.year_ce().1 as i32, dt.month0() + 1, dt.day()).unwrap()
}

/// Some snippets contain weird text between < />, we should remove it
pub fn clean_snippet(snippet: String) -> String {
    let re = Regex::new(r"<[^>]*\/?>").unwrap();
    re.replace_all(&snippet, "").to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// An image for an article that may be passed to the frontend
pub struct FrontendImage {
    pub url: String,
    pub caption: Option<String>,
}

/// The "important" information from an article and it's associated stuff that we will eventually pass to frontend
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrontendArticle {
    pub uri: String,
    pub web_url: String,
    pub headline_main: String,
    pub snippet: String,
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub image: Option<FrontendImage>,
    pub print_section: Option<String>,
    pub document_type: String,
    pub news_desk: String,
    pub type_of_material: String,
}
