use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDate};

pub mod mydrant;
pub mod nyt;
pub mod pg;

pub const START_YEAR: u32 = 1980;
pub const END_YEAR: u32 = 2010; // inclusive

pub fn get_date(year: u32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year as i32, month, 1).unwrap()
}

pub fn get_json_path(date: NaiveDate) -> PathBuf {
    Path::new("scrapes").join(format!("{}_{}.json", date.year_ce().1, date.month0() + 1))
}
