//! The `api-status` widget: what status.claude.com says about the
//! Claude API, and how that reads on the status line.
//!
//! The network and filesystem work lives in [`io`], which coverage
//! ignores; everything here is pure and tested.

use serde::{Deserialize, Serialize};

use super::theme::{GREEN, RED, RESET, YELLOW, label};

mod io;

/// How long a successfully fetched indicator is served without
/// touching the network.
const FRESH_TTL_SECS: u64 = 120;
/// How long to wait after a failed attempt before trying again. Without
/// this, an outage would make every single status-line render pay the
/// full connect timeout.
const RETRY_COOLDOWN_SECS: u64 = 30;

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

fn render_api_health(health: &ApiHealth) -> String {
    let (text, color) = match health {
        ApiHealth::Current(indicator) => indicator_text(indicator),
        // Trailing "~": last known value, status page unreachable.
        ApiHealth::Stale(indicator) => {
            let (text, color) = indicator_text(indicator);
            return format!("{} {color}{text}~{RESET}", label("api"));
        }
        ApiHealth::Unknown => ("unknown", YELLOW),
    };
    format!("{} {color}{text}{RESET}", label("api"))
}

fn indicator_text(indicator: &str) -> (&'static str, &'static str) {
    match indicator {
        "none" => ("ok", GREEN),
        "minor" => ("degraded", YELLOW),
        "major" => ("outage", RED),
        "critical" => ("critical", RED),
        _ => ("unknown", YELLOW),
    }
}

/// Render the `api-status` widget, or `None` for any other name.
/// Unlike most widgets this never renders empty — see [`ApiHealth`].
pub fn render(name: &str) -> Option<String> {
    if name != "api-status" {
        return None;
    }
    Some(render_api_health(&io::get_api_status()))
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

    #[test]
    fn api_health_renders_each_indicator() {
        let cases = [
            ("none", "ok", GREEN),
            ("minor", "degraded", YELLOW),
            ("major", "outage", RED),
            ("critical", "critical", RED),
            ("something-new", "unknown", YELLOW),
        ];
        for (indicator, text, color) in cases {
            let out =
                render_api_health(&ApiHealth::Current(indicator.to_string()));
            assert!(out.contains(text), "{indicator} -> {out}");
            assert!(out.contains(color), "{indicator} -> {out}");
        }
    }

    /// The outage bug: an unreachable status page used to hide the
    /// widget entirely, which reads as "no problem" rather than
    /// "no answer".
    #[test]
    fn api_health_unknown_still_renders() {
        let out = render_api_health(&ApiHealth::Unknown);
        assert!(out.contains("api"));
        assert!(out.contains("unknown"));
        assert!(out.contains(YELLOW));
    }

    #[test]
    fn api_health_stale_is_marked() {
        let out = render_api_health(&ApiHealth::Stale("major".to_string()));
        assert!(out.contains("outage~"));
        assert!(out.contains(RED));
    }

    /// Guards the dispatch chain: a foreign name must be declined
    /// *before* any network call is attempted.
    #[test]
    fn foreign_widget_name_is_declined() {
        assert_eq!(render("model"), None);
    }
}
