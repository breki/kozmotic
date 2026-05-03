use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use serde::Deserialize;

#[derive(Deserialize, Default)]
struct SessionData {
    #[serde(default, deserialize_with = "null_as_default")]
    model: ModelData,
    #[serde(default, deserialize_with = "null_as_default")]
    context_window: ContextData,
    #[serde(default, deserialize_with = "null_as_default")]
    cost: CostData,
    #[serde(default, deserialize_with = "null_as_default")]
    rate_limits: RateLimitsData,
    #[serde(default, deserialize_with = "null_as_default")]
    vim: VimData,
    #[serde(default, deserialize_with = "null_as_default")]
    workspace: WorkspaceData,
    #[serde(default, deserialize_with = "null_as_default")]
    session_id: String,
    #[serde(default, deserialize_with = "null_as_default")]
    agent: AgentData,
    #[serde(default, deserialize_with = "null_as_default")]
    worktree: WorktreeData,
}

#[derive(Deserialize, Default)]
struct ModelData {
    #[serde(default, deserialize_with = "null_as_default")]
    display_name: String,
}

#[derive(Deserialize, Default)]
struct ContextData {
    #[serde(default)]
    used_percentage: f64,
    #[serde(default)]
    total_input_tokens: u64,
    #[serde(default)]
    total_output_tokens: u64,
}

#[derive(Deserialize, Default)]
struct CostData {
    #[serde(default)]
    total_cost_usd: f64,
    #[serde(default)]
    total_duration_ms: u64,
    #[serde(default)]
    total_api_duration_ms: u64,
    #[serde(default)]
    total_lines_added: u64,
    #[serde(default)]
    total_lines_removed: u64,
}

#[derive(Deserialize, Default)]
struct RateLimitsData {
    #[serde(default)]
    five_hour: RateLimitBucket,
    #[serde(default)]
    seven_day: RateLimitBucket,
}

#[derive(Deserialize, Default)]
struct RateLimitBucket {
    #[serde(default)]
    used_percentage: f64,
    /// Unix timestamp (seconds) when the bucket resets. 0 when absent.
    /// Claude Code may send either an integer epoch or an RFC3339 string.
    #[serde(default, deserialize_with = "deserialize_resets_at")]
    resets_at: i64,
}

fn deserialize_resets_at<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(n)) => Ok(n.as_i64().unwrap_or(0)),
        Some(Value::String(s)) => Ok(parse_rfc3339(&s).unwrap_or(0)),
        _ => Ok(0),
    }
}

/// Deserialize a value, treating JSON `null` as the type's default.
/// Claude Code sends `null` for optional fields like `resets_at` or
/// `worktree` instead of omitting them, which would otherwise fail
/// on non-`Option` fields.
fn null_as_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Deserialize, Default)]
struct VimData {
    #[serde(default, deserialize_with = "null_as_default")]
    mode: String,
}

#[derive(Deserialize, Default)]
struct WorkspaceData {
    #[serde(default, deserialize_with = "null_as_default")]
    current_dir: String,
}

#[derive(Deserialize, Default)]
struct AgentData {
    #[serde(default, deserialize_with = "null_as_default")]
    name: String,
}

#[derive(Deserialize, Default)]
struct WorktreeData {
    #[serde(default, deserialize_with = "null_as_default")]
    name: String,
}

pub struct StatusLineArgs {
    pub show: String,
    pub separator: String,
}

pub fn handle_status_line(args: StatusLineArgs) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() || input.trim().is_empty() {
        report_error("no input on stdin");
        return ExitCode::FAILURE;
    }

    let data: SessionData = match serde_json::from_str(&input) {
        Ok(d) => d,
        Err(e) => {
            report_error(&format!("invalid JSON: {e}"));
            return ExitCode::FAILURE;
        }
    };

    // Support multi-line: split on ";" to get lines
    let lines: Vec<&str> = args.show.split(';').collect();
    for line_spec in &lines {
        let widgets: Vec<&str> = line_spec.split(',').map(|s| s.trim()).collect();
        let parts: Vec<String> = widgets
            .iter()
            .filter_map(|w| render_widget(w, &data))
            .collect();
        if !parts.is_empty() {
            println!("{}", parts.join(&args.separator));
        }
    }

    ExitCode::SUCCESS
}

/// Print a diagnostic that's visible in the Claude Code status line
/// (stdout) and also logged to stderr for terminal invocations. A
/// silent failure makes the status line disappear entirely, which is
/// almost always worse than showing what went wrong.
fn report_error(msg: &str) {
    eprintln!("Error: {msg}");
    println!("{RED}status-line: {msg}{RESET}");
}

fn format_duration_ms(ms: u64) -> String {
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
fn parse_rfc3339(s: &str) -> Option<i64> {
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
    let m = m as i64;
    let d = d as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Format a Unix timestamp (seconds) as a local datetime using the
/// given `strftime`-style pattern. Returns `None` when the timestamp
/// is absent (`0`) or out of range.
fn format_reset(resets_at: i64, fmt: &str) -> Option<String> {
    if resets_at == 0 {
        return None;
    }
    let dt: DateTime<Local> = DateTime::from_timestamp(resets_at, 0)?.with_timezone(&Local);
    Some(dt.format(fmt).to_string())
}

fn format_tokens(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        format!("{count}")
    }
}

fn git_branch() -> Option<String> {
    Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn git_status_counts() -> Option<(usize, usize)> {
    let staged = Command::new("git")
        .args(["diff", "--cached", "--numstat"])
        .output()
        .ok()?;
    let modified = Command::new("git")
        .args(["diff", "--numstat"])
        .output()
        .ok()?;
    if !staged.status.success() {
        return None;
    }
    let staged_count = String::from_utf8_lossy(&staged.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .count();
    let modified_count = String::from_utf8_lossy(&modified.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .count();
    Some((staged_count, modified_count))
}

struct GitFileCounts {
    staged: usize,
    modified: usize,
    new: usize,
    deleted: usize,
}

fn git_file_counts() -> Option<GitFileCounts> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut staged = 0;
    let mut modified = 0;
    let mut new = 0;
    let mut deleted = 0;
    for line in stdout.lines() {
        if line.len() < 2 {
            continue;
        }
        let index = line.as_bytes()[0];
        let worktree = line.as_bytes()[1];
        // Untracked
        if index == b'?' {
            new += 1;
            continue;
        }
        // Index (staged) changes
        match index {
            b'A' => staged += 1,
            b'M' => staged += 1,
            b'D' => {
                staged += 1;
                deleted += 1;
            }
            b'R' => staged += 1,
            _ => {}
        }
        // Worktree (unstaged) changes
        match worktree {
            b'M' => modified += 1,
            b'D' => {
                deleted += 1;
            }
            _ => {}
        }
    }
    Some(GitFileCounts {
        staged,
        modified,
        new,
        deleted,
    })
}

const STATUS_CACHE_FILE: &str = "kozmotic-api-status.json";
const STATUS_CACHE_TTL_SECS: u64 = 120; // 2 minutes
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

fn get_api_status() -> Option<String> {
    if let Some(cached) = read_cached_status() {
        return Some(cached);
    }
    fetch_and_cache_status()
}

const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

fn label(name: &str) -> String {
    format!("{DIM}{name}{RESET}")
}

fn git_ahead_behind() -> Option<(usize, usize)> {
    let output = Command::new("git")
        .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split('\t').collect();
    if parts.len() == 2 {
        let ahead = parts[0].parse().unwrap_or(0);
        let behind = parts[1].parse().unwrap_or(0);
        Some((ahead, behind))
    } else {
        None
    }
}

fn render_rate_limit(lbl: &str, bucket: &RateLimitBucket, reset_fmt: &str) -> Option<String> {
    let pct = bucket.used_percentage;
    let has_reset = bucket.resets_at != 0;
    if pct <= 0.0 && !has_reset {
        return None;
    }
    let mut out = format!("{} {pct:.0}%", label(lbl));
    if let Some(when) = format_reset(bucket.resets_at, reset_fmt) {
        out.push_str(&format!(" (→{when})"));
    }
    Some(out)
}

fn render_widget(name: &str, data: &SessionData) -> Option<String> {
    match name {
        "model" => {
            if data.model.display_name.is_empty() {
                None
            } else {
                Some(data.model.display_name.clone())
            }
        }
        "context" => {
            let pct = data.context_window.used_percentage;
            let color = if pct >= 80.0 {
                RED
            } else if pct >= 50.0 {
                YELLOW
            } else {
                GREEN
            };
            Some(format!("{} {color}{pct:.1}%{RESET}", label("ctx")))
        }
        "cost" => {
            let cost = data.cost.total_cost_usd;
            Some(format!("{} ${cost:.2}", label("cost")))
        }
        "lines" => {
            let added = data.cost.total_lines_added;
            let removed = data.cost.total_lines_removed;
            Some(format!("{GREEN}+{added}{RESET}/{RED}-{removed}{RESET}"))
        }
        "duration" => {
            let ms = data.cost.total_duration_ms;
            Some(format!("{} {}", label("time"), format_duration_ms(ms)))
        }
        "api-duration" => {
            let ms = data.cost.total_api_duration_ms;
            Some(format!("{} {}", label("api"), format_duration_ms(ms)))
        }
        "tokens" => {
            let input = data.context_window.total_input_tokens;
            let output = data.context_window.total_output_tokens;
            Some(format!(
                "{} {} in / {} out",
                label("tok"),
                format_tokens(input),
                format_tokens(output)
            ))
        }
        "git-branch" => git_branch().map(|b| format!("{CYAN}{b}{RESET}")),
        "git-ahead" => {
            let (ahead, behind) = git_ahead_behind()?;
            if ahead == 0 && behind == 0 {
                None
            } else {
                let mut parts = Vec::new();
                if ahead > 0 {
                    parts.push(format!("{GREEN}↑{ahead}{RESET}"));
                }
                if behind > 0 {
                    parts.push(format!("{RED}↓{behind}{RESET}"));
                }
                Some(parts.join(" "))
            }
        }
        "git-files" => {
            let counts = git_file_counts()?;
            let mut parts = Vec::new();
            if counts.staged > 0 {
                parts.push(format!("{GREEN}{}staged{RESET}", counts.staged));
            }
            if counts.modified > 0 {
                parts.push(format!("{YELLOW}{}mod{RESET}", counts.modified));
            }
            if counts.new > 0 {
                parts.push(format!("{CYAN}{}new{RESET}", counts.new));
            }
            if counts.deleted > 0 {
                parts.push(format!("{RED}{}del{RESET}", counts.deleted));
            }
            if parts.is_empty() {
                Some(format!("{} (clean)", label("git")))
            } else {
                Some(format!("{} {}", label("git"), parts.join(" ")))
            }
        }
        "git-status" => {
            let (staged, modified) = git_status_counts()?;
            if staged == 0 && modified == 0 {
                None
            } else {
                let mut parts = Vec::new();
                if staged > 0 {
                    parts.push(format!("{GREEN}+{staged}{RESET}"));
                }
                if modified > 0 {
                    parts.push(format!("{YELLOW}~{modified}{RESET}"));
                }
                Some(parts.join(" "))
            }
        }
        "directory" => {
            if data.workspace.current_dir.is_empty() {
                None
            } else {
                let dir = &data.workspace.current_dir;
                let name = dir.rsplit(['/', '\\']).next().unwrap_or(dir);
                Some(name.to_string())
            }
        }
        "session" => {
            if data.session_id.is_empty() {
                None
            } else {
                let short: String = data.session_id.chars().take(8).collect();
                Some(format!("{} {short}", label("sid")))
            }
        }
        "rate-limit" => {
            let bucket = &data.rate_limits.five_hour;
            render_rate_limit("5h", bucket, "%H:%M")
        }
        "rate-limit-7d" => {
            let bucket = &data.rate_limits.seven_day;
            render_rate_limit("7d", bucket, "%a %H:%M")
        }
        "vim" => {
            if data.vim.mode.is_empty() {
                None
            } else {
                Some(data.vim.mode.clone())
            }
        }
        "worktree" => {
            if data.worktree.name.is_empty() {
                None
            } else {
                Some(format!("{} {}", label("wt"), data.worktree.name))
            }
        }
        "agent" => {
            if data.agent.name.is_empty() {
                None
            } else {
                Some(format!("{} {}", label("agent"), data.agent.name))
            }
        }
        "api-status" => {
            let indicator = get_api_status()?;
            let (symbol, color) = match indicator.as_str() {
                "none" => ("ok", GREEN),
                "minor" => ("degraded", YELLOW),
                "major" => ("outage", RED),
                "critical" => ("critical", RED),
                _ => ("?", YELLOW),
            };
            Some(format!("{} {color}{symbol}{RESET}", label("api")))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_under_one_hour() {
        assert_eq!(format_duration_ms(0), "0m 0s");
        assert_eq!(format_duration_ms(1_500), "0m 1s");
        assert_eq!(format_duration_ms(65_000), "1m 5s");
        assert_eq!(format_duration_ms(59 * 60_000 + 59_000), "59m 59s");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration_ms(60 * 60_000), "1h 0m");
        assert_eq!(format_duration_ms(90 * 60_000), "1h 30m");
        assert_eq!(format_duration_ms(23 * 3_600_000 + 59 * 60_000), "23h 59m");
    }

    #[test]
    fn format_duration_days() {
        assert_eq!(format_duration_ms(24 * 3_600_000), "1d 0h");
        assert_eq!(format_duration_ms(3096 * 60_000 + 2_000), "2d 3h");
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
        assert_eq!(format_reset(0, "%H:%M"), None);
    }

    #[test]
    fn format_reset_renders_local() {
        // Just verify it produces a non-empty string in the expected shape.
        let out = format_reset(1_776_711_600, "%H:%M").expect("should render");
        assert_eq!(out.len(), 5);
        assert_eq!(&out[2..3], ":");
    }

    #[test]
    fn accepts_null_resets_at() {
        let json = r#"{"rate_limits":{"five_hour":{"used_percentage":73.2,"resets_at":null}}}"#;
        let data: SessionData = serde_json::from_str(json).expect("should parse");
        assert_eq!(data.rate_limits.five_hour.used_percentage, 73.2);
        assert_eq!(data.rate_limits.five_hour.resets_at, 0);
    }

    #[test]
    fn accepts_integer_resets_at() {
        let json = r#"{"rate_limits":{"five_hour":{"used_percentage":51,"resets_at":1776711600}}}"#;
        let data: SessionData = serde_json::from_str(json).expect("should parse");
        assert_eq!(data.rate_limits.five_hour.resets_at, 1_776_711_600);
    }

    #[test]
    fn accepts_rfc3339_resets_at() {
        let json = r#"{"rate_limits":{"five_hour":{"resets_at":"2026-04-20T00:00:00Z"}}}"#;
        let data: SessionData = serde_json::from_str(json).expect("should parse");
        assert_eq!(data.rate_limits.five_hour.resets_at, 1_776_643_200);
    }

    #[test]
    fn render_rate_limit_hidden_when_empty() {
        let bucket = RateLimitBucket::default();
        assert_eq!(render_rate_limit("5h", &bucket, "%H:%M"), None);
    }

    #[test]
    fn render_rate_limit_shown_with_only_reset() {
        let bucket = RateLimitBucket {
            used_percentage: 0.0,
            resets_at: 4_102_444_800,
        };
        let out = render_rate_limit("5h", &bucket, "%H:%M").expect("should render");
        assert!(out.contains("0%"));
        assert!(out.contains("→"));
    }
}
