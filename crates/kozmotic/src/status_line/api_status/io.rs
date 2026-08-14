// Network- and filesystem-bound half of the api-status widget:
// fetching status.claude.com and caching what it said. Excluded from
// coverage because it requires network access and a live status page.
// Anything that can be decided without I/O belongs in the parent
// module instead, where it is measured.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::{ApiHealth, CacheDecision, CacheRecord, decide};

const STATUS_CACHE_FILE: &str = "kozmotic-api-status.json";
const CONNECT_TIMEOUT: Duration = Duration::from_millis(1500);
const GLOBAL_TIMEOUT: Duration = Duration::from_millis(2500);
const STATUS_URL: &str = "https://status.claude.com/api/v2/summary.json";

#[derive(serde::Deserialize)]
struct StatusPageResponse {
    status: StatusPageStatus,
}

#[derive(serde::Deserialize)]
struct StatusPageStatus {
    indicator: String,
}

fn status_cache_path() -> PathBuf {
    std::env::temp_dir().join(STATUS_CACHE_FILE)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn read_cache() -> Option<CacheRecord> {
    let raw = std::fs::read_to_string(status_cache_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_cache(record: &CacheRecord) {
    if let Ok(raw) = serde_json::to_string(record) {
        let _ = std::fs::write(status_cache_path(), raw);
    }
}

fn fetch_indicator() -> Option<String> {
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(GLOBAL_TIMEOUT))
        .build();
    let body: String = config
        .new_agent()
        .get(STATUS_URL)
        .call()
        .ok()?
        .into_body()
        .read_to_string()
        .ok()?;
    let parsed: StatusPageResponse = serde_json::from_str(&body).ok()?;
    Some(parsed.status.indicator)
}

pub fn get_api_status() -> ApiHealth {
    let record = read_cache();
    let now = now_secs();
    let previous = match decide(record.as_ref(), now) {
        CacheDecision::Serve(health) => return health,
        CacheDecision::Fetch(previous) => previous,
    };

    if let Some(indicator) = fetch_indicator() {
        write_cache(&CacheRecord {
            indicator: Some(indicator.clone()),
            fetched_at: now,
            checked_at: now,
        });
        ApiHealth::Current(indicator)
    } else {
        // Record the failed attempt so the next render doesn't pay the
        // timeout again, but keep the last known indicator.
        write_cache(&CacheRecord {
            indicator: record.as_ref().and_then(|r| r.indicator.clone()),
            fetched_at: record.as_ref().map_or(0, |r| r.fetched_at),
            checked_at: now,
        });
        previous
    }
}
