use chrono::{Datelike, NaiveDate};
use cyclicism::get_date;
use std::{env, time::Duration};

enum MonthStatus {
    AlreadyExists,
    DownloadFailed,
    WriteFailed,
    Downloaded,
}

/// Gets the json and writes it to the file. Does NOT sleep.
/// Returns Ok(false) if the file already exists.
/// Returns Ok(true) if we successfully
async fn handle_month(date: NaiveDate) -> MonthStatus {
    let api_key = env::var("NYT_API_KEY").unwrap();
    let path = cyclicism::get_json_path(date);
    if path.exists() {
        return MonthStatus::AlreadyExists;
    }
    let Ok(resp) = reqwest::get(format!(
        "https://api.nytimes.com/svc/archive/v1/{}/{}.json?api-key={}",
        date.year_ce().1,
        date.month0() + 1,
        api_key
    ))
    .await
    else {
        return MonthStatus::DownloadFailed;
    };
    let Ok(body) = resp.text().await else {
        return MonthStatus::DownloadFailed;
    };
    if tokio::fs::write(cyclicism::get_json_path(date), body)
        .await
        .is_err()
    {
        return MonthStatus::WriteFailed;
    }
    MonthStatus::Downloaded
}

async fn scrape_data() {
    // Don't get rate-limited
    const SLEEP_SECS: u64 = 15;
    let mut total_retries_left = 100;
    for year in cyclicism::START_YEAR..=cyclicism::END_YEAR {
        let mut month = 1;
        while month <= 12 {
            if total_retries_left <= 0 {
                panic!("Ran out of retries trying to scrape data :/");
            }
            let date = get_date(year, month);
            match handle_month(date).await {
                MonthStatus::AlreadyExists => {
                    month += 1;
                }
                MonthStatus::DownloadFailed | MonthStatus::WriteFailed => {
                    tokio::time::sleep(Duration::from_secs(SLEEP_SECS)).await;
                    total_retries_left -= 1;
                }
                MonthStatus::Downloaded => {
                    tokio::time::sleep(Duration::from_secs(SLEEP_SECS)).await;
                    month += 1;
                }
            }
        }
        println!("Finished {year}");
    }
}

#[tokio::main]
async fn main() {
    scrape_data().await;
}
