//! Value formatting shared by the widgets: durations, ages, token
//! counts, and the timestamp parsing behind reset times.

use chrono::{DateTime, Local};

/// Strip control characters from a string that came from outside the
/// program, and bound its length.
///
/// The status line is redrawn continuously, and several widgets
/// interpolate session-supplied text — model name, directory,
/// worktree, agent, branch. A POSIX filename may contain `\x1b`, so a
/// directory named with an embedded CSI sequence would otherwise have
/// that sequence re-emitted to the terminal on every render, leaving
/// colour or cursor state changed and making `display_width`'s
/// measurement disagree with what is actually shown.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() && *c != '\u{7f}')
        .take(MAX_FIELD_CHARS)
        .collect()
}

/// Upper bound on any single session-supplied field.
const MAX_FIELD_CHARS: usize = 120;

/// Compact age formatter with minute granularity: "5m", "1h 5m",
/// "2d 3h". Used by `last-commit` where seconds are too noisy.
pub fn age_compact(secs: u64) -> String {
    let mins = secs / 60;
    let hours = mins / 60;
    if hours >= 24 {
        let days = hours / 24;
        let h = hours % 24;
        format!("{days}d {h}h")
    } else if hours > 0 {
        let m = mins % 60;
        format!("{hours}h {m}m")
    } else {
        format!("{mins}m")
    }
}

pub fn duration_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let total_mins = total_secs / 60;
    let secs = total_secs % 60;
    let mins = total_mins % 60;
    let hours = total_mins / 60;
    if hours >= 24 {
        let days = hours / 24;
        let hours = hours % 24;
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m {secs}s")
    }
}

/// Parse an RFC3339 UTC timestamp into a Unix timestamp (seconds).
///
/// Accepts forms like `2026-04-20T15:04:05Z`, `...T15:04:05.123Z`,
/// or with a `+00:00` / `-HH:MM` offset. Non-UTC offsets are applied.
/// Parse an RFC3339 timestamp to a Unix epoch second.
///
/// Delegates to chrono, which is already a direct dependency (the
/// envelope's timestamp uses it). A hand-rolled parser here meant
/// ~70 lines of offset arithmetic and a civil-days implementation to
/// maintain, for a calculation the dependency graph already had.
///
/// The one accommodation: Claude Code has been observed emitting a
/// space instead of `T` as the date/time separator, which is legal
/// per RFC3339 section 5.6 but which chrono's strict parser rejects,
/// so it is normalised first.
pub fn parse_rfc3339(s: &str) -> Option<i64> {
    let trimmed = s.trim();
    let normalised = match trimmed.as_bytes().get(10) {
        Some(b' ') => {
            let mut owned = trimmed.to_string();
            owned.replace_range(10..11, "T");
            std::borrow::Cow::Owned(owned)
        }
        _ => std::borrow::Cow::Borrowed(trimmed),
    };
    DateTime::parse_from_rfc3339(&normalised)
        .ok()
        .map(|dt| dt.timestamp())
}

/// Format a Unix timestamp (seconds) as a local datetime using the
/// given `strftime`-style pattern. Returns `None` when the timestamp
/// is absent (`0`) or out of range.
pub fn reset_time(resets_at: i64, fmt: &str) -> Option<String> {
    if resets_at == 0 {
        return None;
    }
    let dt: DateTime<Local> =
        DateTime::from_timestamp(resets_at, 0)?.with_timezone(&Local);
    Some(dt.format(fmt).to_string())
}

pub fn tokens(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        format!("{count}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_escape_sequences() {
        // A directory named with an embedded CSI sequence would
        // otherwise re-emit it on every render.
        assert_eq!(sanitize("proj\x1b[2J\x1b[1;31m"), "proj[2J[1;31m");
        assert_eq!(sanitize("a\nb\tc\r"), "abc");
        assert_eq!(sanitize("plain-name"), "plain-name");
    }

    #[test]
    fn sanitize_bounds_length() {
        let long = "x".repeat(MAX_FIELD_CHARS * 2);
        assert_eq!(sanitize(&long).chars().count(), MAX_FIELD_CHARS);
    }

    #[test]
    fn sanitize_keeps_non_ascii_text() {
        assert_eq!(sanitize("café-日本"), "café-日本");
    }

    #[test]
    fn format_duration_under_one_hour() {
        assert_eq!(duration_ms(0), "0m 0s");
        assert_eq!(duration_ms(1_500), "0m 1s");
        assert_eq!(duration_ms(65_000), "1m 5s");
        assert_eq!(duration_ms(59 * 60_000 + 59_000), "59m 59s");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(duration_ms(60 * 60_000), "1h 0m");
        assert_eq!(duration_ms(90 * 60_000), "1h 30m");
        assert_eq!(duration_ms(23 * 3_600_000 + 59 * 60_000), "23h 59m");
    }

    #[test]
    fn format_duration_days() {
        assert_eq!(duration_ms(24 * 3_600_000), "1d 0h");
        assert_eq!(duration_ms(3096 * 60_000 + 2_000), "2d 3h");
    }

    #[test]
    fn format_age_compact_floors_to_minutes() {
        assert_eq!(age_compact(0), "0m");
        assert_eq!(age_compact(45), "0m");
        assert_eq!(age_compact(60), "1m");
        assert_eq!(age_compact(12 * 60 + 59), "12m");
        assert_eq!(age_compact(60 * 60), "1h 0m");
        assert_eq!(age_compact(2 * 3600 + 15 * 60), "2h 15m");
        assert_eq!(age_compact(24 * 3600), "1d 0h");
        assert_eq!(age_compact(3 * 86400 + 4 * 3600 + 30 * 60), "3d 4h");
    }

    #[test]
    fn format_tokens_scales() {
        assert_eq!(tokens(0), "0");
        assert_eq!(tokens(999), "999");
        assert_eq!(tokens(1_500), "1.5k");
        assert_eq!(tokens(1_500_000), "1.5M");
    }

    #[test]
    fn parse_rfc3339_utc_z() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339("1970-01-01T00:00:01Z"), Some(1));
        assert_eq!(parse_rfc3339("2026-04-20T00:00:00Z"), Some(1_776_643_200));
    }

    #[test]
    fn parse_rfc3339_fractional() {
        assert_eq!(
            parse_rfc3339("2026-04-20T00:00:00.123Z"),
            Some(1_776_643_200)
        );
    }

    #[test]
    fn parse_rfc3339_offset() {
        // 2026-04-20T02:00:00+02:00 == 2026-04-20T00:00:00Z
        assert_eq!(
            parse_rfc3339("2026-04-20T02:00:00+02:00"),
            Some(1_776_643_200)
        );
        assert_eq!(
            parse_rfc3339("2026-04-19T22:00:00-02:00"),
            Some(1_776_643_200)
        );
    }

    #[test]
    fn parse_rfc3339_invalid() {
        assert_eq!(parse_rfc3339(""), None);
        assert_eq!(parse_rfc3339("not a date"), None);
        assert_eq!(parse_rfc3339("2026/04/20T00:00:00Z"), None);
    }

    #[test]
    fn format_reset_absent() {
        assert_eq!(reset_time(0, "%H:%M"), None);
    }

    #[test]
    fn format_reset_renders_local() {
        // Just verify it produces a non-empty string in the expected shape.
        let out = reset_time(1_776_711_600, "%H:%M").expect("should render");
        assert_eq!(out.len(), 5);
        assert_eq!(&out[2..3], ":");
    }
}
