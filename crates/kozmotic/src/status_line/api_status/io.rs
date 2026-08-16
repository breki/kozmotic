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

/// Where the cached indicator lives, or `None` when there is no home
/// directory to put it in.
///
/// Under `~/.claude/` rather than the system temp directory: a fixed
/// name in a world-writable directory lets any other user on the host
/// pre-create the file, after which our writes fail silently and every
/// render pays the full `GLOBAL_TIMEOUT` — defeating the retry
/// cooldown this cache exists to provide — or we serve an
/// attacker-chosen indicator during a real outage. With no home
/// directory we simply do not cache, which is slower but never wrong.
fn status_cache_path() -> Option<PathBuf> {
    Some(
        crate::self_install::home_dir()?
            .join(".claude")
            .join(STATUS_CACHE_FILE),
    )
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn read_cache() -> Option<CacheRecord> {
    let raw = std::fs::read_to_string(status_cache_path()?).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Write the cache atomically.
///
/// A bare `fs::write` truncates before writing, so a concurrent
/// render — Claude Code re-renders the status line every turn — can
/// read a half-written file, parse nothing, and go to the network.
/// Writing to a pid-qualified temp file and renaming makes the
/// swap atomic for readers.
fn write_cache(record: &CacheRecord) {
    let Ok(raw) = serde_json::to_string(record) else {
        return;
    };
    let Some(path) = status_cache_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&tmp, raw).is_ok()
        && std::fs::rename(&tmp, &path).is_err()
    {
        let _ = std::fs::remove_file(&tmp);
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
