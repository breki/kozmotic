// Network-bound status.claude.com fetching. Excluded from
// coverage because it requires network access and a live
// status page.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::Deserialize;

const STATUS_CACHE_FILE: &str = "kozmotic-api-status.json";
const STATUS_CACHE_TTL_SECS: u64 = 120;
const STATUS_URL: &str = "https://status.claude.com/api/v2/summary.json";

#[derive(Deserialize)]
struct StatusPageResponse {
    status: StatusPageStatus,
}

#[derive(Deserialize)]
struct StatusPageStatus {
    indicator: String,
}

fn status_cache_path() -> PathBuf {
    std::env::temp_dir().join(STATUS_CACHE_FILE)
}

fn read_cached_status() -> Option<String> {
    let path = status_cache_path();
    let metadata = std::fs::metadata(&path).ok()?;
    let age = SystemTime::now()
        .duration_since(metadata.modified().ok()?)
        .ok()?;
    if age.as_secs() > STATUS_CACHE_TTL_SECS {
        return None;
    }
    std::fs::read_to_string(&path).ok()
}

fn fetch_and_cache_status() -> Option<String> {
    let body: String = ureq::get(STATUS_URL)
        .call()
        .ok()?
        .into_body()
        .read_to_string()
        .ok()?;
    let parsed: StatusPageResponse = serde_json::from_str(&body).ok()?;
    let indicator = parsed.status.indicator;
    let _ = std::fs::write(status_cache_path(), &indicator);
    Some(indicator)
}

pub fn get_api_status() -> Option<String> {
    if let Some(cached) = read_cached_status() {
        return Some(cached);
    }
    fetch_and_cache_status()
}
