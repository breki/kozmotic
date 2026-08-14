//! Value formatting shared by the widgets: durations, ages, token
//! counts, and the timestamp parsing behind reset times.

use chrono::{DateTime, Local};

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
pub fn parse_rfc3339(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let b = s.as_bytes();
    // YYYY-MM-DDTHH:MM:SS
    if b[4] != b'-' || b[7] != b'-' || (b[10] != b'T' && b[10] != b' ') {
        return None;
    }
    if b[13] != b':' || b[16] != b':' {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let hour: i64 = s.get(11..13)?.parse().ok()?;
    let minute: i64 = s.get(14..16)?.parse().ok()?;
    let second: i64 = s.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    // Skip optional fractional seconds.
    let mut i = 19;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
    }

    // Parse offset.
    let mut offset_secs: i64 = 0;
    if i < b.len() {
        match b[i] {
            b'Z' | b'z' => {}
            b'+' | b'-' => {
                let sign: i64 = if b[i] == b'-' { -1 } else { 1 };
                let oh: i64 = s.get(i + 1..i + 3)?.parse().ok()?;
                let om: i64 = if b.len() > i + 3 && b[i + 3] == b':' {
                    s.get(i + 4..i + 6)?.parse().ok()?
                } else {
                    s.get(i + 3..i + 5)?.parse().ok()?
                };
                offset_secs = sign * (oh * 3600 + om * 60);
            }
            _ => return None,
        }
    }

    let days = days_from_civil(year, month, day);
    let epoch = days * 86400 + hour * 3600 + minute * 60 + second - offset_secs;
    Some(epoch)
}

/// Howard Hinnant's days-from-civil algorithm. Returns days since
/// 1970-01-01 (can be negative).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(m);
    let d = i64::from(d);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
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
