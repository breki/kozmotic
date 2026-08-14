// Network-bound status.claude.com fetching. Excluded from
// coverage because it requires network access and a live
// status page; the pure cache-decision logic below is still
// unit-tested.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

const STATUS_CACHE_FILE: &str = "kozmotic-api-status.json";
/// How long a successfully fetched indicator is served without
/// touching the network.
const FRESH_TTL_SECS: u64 = 120;
/// How long to wait after a failed attempt before trying again. Without
/// this, an outage would make every single status-line render pay the
/// full connect timeout.
const RETRY_COOLDOWN_SECS: u64 = 30;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(1500);
const GLOBAL_TIMEOUT: Duration = Duration::from_millis(2500);
const STATUS_URL: &str = "https://status.claude.com/api/v2/summary.json";

/// What we know about the Claude API's health right now.
///
/// There is deliberately no "nothing to show" case: an unreachable
/// status page is itself worth reporting, and a widget that silently
/// disappears is indistinguishable from one that was never configured.
#[derive(Clone, Debug, PartialEq)]
pub enum ApiHealth {
    /// A freshly fetched (or still-fresh cached) status-page indicator.
    Current(String),
    /// The last known indicator, served from an expired cache because
    /// the status page could not be reached.
    Stale(String),
    /// The status page could not be reached and nothing was cached.
    Unknown,
}

#[derive(Deserialize)]
struct StatusPageResponse {
    status: StatusPageStatus,
}

#[derive(Deserialize)]
struct StatusPageStatus {
    indicator: String,
}

/// On-disk cache. `checked_at` records every attempt (successful or
/// not) so failures are rate-limited too.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct CacheRecord {
    #[serde(default)]
    indicator: Option<String>,
    #[serde(default)]
    fetched_at: u64,
    #[serde(default)]
    checked_at: u64,
}

/// What to do with a cache record, given the current time.
#[derive(Debug, PartialEq)]
enum CacheDecision {
    /// The cache answers the question; no network call needed.
    Serve(ApiHealth),
    /// Hit the network, falling back to this if the call fails.
    Fetch(ApiHealth),
}

fn status_cache_path() -> PathBuf {
    std::env::temp_dir().join(STATUS_CACHE_FILE)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The fallback health implied by a (possibly absent) cache record.
fn fallback(record: Option<&CacheRecord>) -> ApiHealth {
    record
        .and_then(|r| r.indicator.clone())
        .map_or(ApiHealth::Unknown, ApiHealth::Stale)
}

fn decide(record: Option<&CacheRecord>, now: u64) -> CacheDecision {
    let Some(rec) = record else {
        return CacheDecision::Fetch(ApiHealth::Unknown);
    };
    if let Some(indicator) = &rec.indicator
        && now.saturating_sub(rec.fetched_at) <= FRESH_TTL_SECS
    {
        return CacheDecision::Serve(ApiHealth::Current(indicator.clone()));
    }
    if now.saturating_sub(rec.checked_at) < RETRY_COOLDOWN_SECS {
        return CacheDecision::Serve(fallback(record));
    }
    CacheDecision::Fetch(fallback(record))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        indicator: Option<&str>,
        fetched: u64,
        checked: u64,
    ) -> CacheRecord {
        CacheRecord {
            indicator: indicator.map(str::to_string),
            fetched_at: fetched,
            checked_at: checked,
        }
    }

    #[test]
    fn no_cache_triggers_fetch_with_unknown_fallback() {
        assert_eq!(
            decide(None, 1_000),
            CacheDecision::Fetch(ApiHealth::Unknown)
        );
    }

    #[test]
    fn fresh_cache_is_served_without_network() {
        let rec = record(Some("none"), 1_000, 1_000);
        assert_eq!(
            decide(Some(&rec), 1_000 + FRESH_TTL_SECS),
            CacheDecision::Serve(ApiHealth::Current("none".into()))
        );
    }

    #[test]
    fn expired_cache_is_refetched_with_stale_fallback() {
        let rec = record(Some("major"), 1_000, 1_000);
        let now = 1_000 + FRESH_TTL_SECS + 1;
        assert_eq!(
            decide(Some(&rec), now),
            CacheDecision::Fetch(ApiHealth::Stale("major".into()))
        );
    }

    #[test]
    fn recent_failure_serves_stale_without_retrying() {
        // Indicator is old, but we attempted (and failed) a moment ago.
        let rec = record(Some("major"), 1_000, 5_000);
        assert_eq!(
            decide(Some(&rec), 5_000 + RETRY_COOLDOWN_SECS - 1),
            CacheDecision::Serve(ApiHealth::Stale("major".into()))
        );
    }

    #[test]
    fn recent_failure_without_history_serves_unknown() {
        let rec = record(None, 0, 5_000);
        assert_eq!(
            decide(Some(&rec), 5_000 + 1),
            CacheDecision::Serve(ApiHealth::Unknown)
        );
    }

    #[test]
    fn cooldown_expiry_allows_another_attempt() {
        let rec = record(None, 0, 5_000);
        assert_eq!(
            decide(Some(&rec), 5_000 + RETRY_COOLDOWN_SECS),
            CacheDecision::Fetch(ApiHealth::Unknown)
        );
    }

    #[test]
    fn cache_record_survives_a_round_trip() {
        let rec = record(Some("minor"), 42, 43);
        let raw = serde_json::to_string(&rec).expect("should serialize");
        let back: CacheRecord =
            serde_json::from_str(&raw).expect("should deserialize");
        assert_eq!(back, rec);
    }

    #[test]
    fn legacy_plain_text_cache_is_ignored() {
        // v1.1.0 wrote the bare indicator string to this file.
        assert!(serde_json::from_str::<CacheRecord>("none").is_err());
    }
}
