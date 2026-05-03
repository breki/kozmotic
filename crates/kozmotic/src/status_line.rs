use std::cell::OnceCell;
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

    let git = GitContext::default();
    // Support multi-line: split on ";" to get lines
    let lines: Vec<&str> = args.show.split(';').collect();
    for line_spec in &lines {
        let widgets: Vec<&str> = line_spec.split(',').map(|s| s.trim()).collect();
        let parts: Vec<String> = widgets
            .iter()
            .filter_map(|w| render_widget(w, &data, &git))
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

/// Compact age formatter with minute granularity: "5m", "1h 5m",
/// "2d 3h". Used by `last-commit` where seconds are too noisy.
fn format_age_compact(secs: u64) -> String {
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

struct GitFileCounts {
    staged: usize,
    modified: usize,
    new: usize,
    deleted: usize,
}

/// Lazily-cached git command results, shared across all git-* widgets
/// in a single status-line invocation. Each underlying `git` process
/// is spawned at most once per render.
#[derive(Default)]
struct GitContext {
    branch: OnceCell<Option<String>>,
    ahead_behind: OnceCell<Option<(usize, usize)>>,
    porcelain: OnceCell<Option<String>>,
    numstat_unstaged: OnceCell<Option<String>>,
    numstat_staged: OnceCell<Option<String>>,
    last_commit: OnceCell<Option<String>>,
}

fn run_git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

impl GitContext {
    fn branch(&self) -> Option<&str> {
        self.branch
            .get_or_init(|| {
                run_git(&["branch", "--show-current"])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .as_deref()
    }

    fn ahead_behind(&self) -> Option<(usize, usize)> {
        *self.ahead_behind.get_or_init(|| {
            let stdout = run_git(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])?;
            let parts: Vec<&str> = stdout.trim().split('\t').collect();
            if parts.len() == 2 {
                Some((parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0)))
            } else {
                None
            }
        })
    }

    fn porcelain(&self) -> Option<&str> {
        self.porcelain
            .get_or_init(|| run_git(&["status", "--porcelain"]))
            .as_deref()
    }

    fn numstat_unstaged(&self) -> Option<&str> {
        self.numstat_unstaged
            .get_or_init(|| run_git(&["diff", "--numstat"]))
            .as_deref()
    }

    fn numstat_staged(&self) -> Option<&str> {
        self.numstat_staged
            .get_or_init(|| run_git(&["diff", "--cached", "--numstat"]))
            .as_deref()
    }

    /// Age of HEAD as a compact duration string with minute
    /// granularity ("12m", "2h 15m", "3d 4h"). Computed from the
    /// author timestamp; returns `None` when not in a repo.
    fn last_commit(&self) -> Option<&str> {
        self.last_commit
            .get_or_init(|| {
                let raw = run_git(&["log", "-1", "--format=%at"])?;
                let ts: i64 = raw.trim().parse().ok()?;
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()?
                    .as_secs() as i64;
                let age_secs = (now - ts).max(0) as u64;
                Some(format_age_compact(age_secs))
            })
            .as_deref()
    }

    fn file_counts(&self) -> Option<GitFileCounts> {
        let stdout = self.porcelain()?;
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
            if index == b'?' {
                new += 1;
                continue;
            }
            match index {
                b'A' | b'M' | b'R' => staged += 1,
                b'D' => {
                    staged += 1;
                    deleted += 1;
                }
                _ => {}
            }
            match worktree {
                b'M' => modified += 1,
                b'D' => deleted += 1,
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

    /// (staged_files_changed, unstaged_files_changed) — counted from
    /// numstat row counts, not porcelain, to match historical behavior.
    fn status_counts(&self) -> Option<(usize, usize)> {
        let staged = self.numstat_staged()?;
        let modified = self.numstat_unstaged()?;
        let staged_count = staged.lines().filter(|l| !l.is_empty()).count();
        let modified_count = modified.lines().filter(|l| !l.is_empty()).count();
        Some((staged_count, modified_count))
    }

    /// Sum added/deleted line counts across both staged and unstaged
    /// changes. Binary files (numstat rows starting with `-`) are
    /// skipped.
    fn diff_lines(&self) -> Option<(usize, usize)> {
        let mut added = 0usize;
        let mut deleted = 0usize;
        for stdout in [self.numstat_unstaged()?, self.numstat_staged()?] {
            for line in stdout.lines() {
                let mut cols = line.split('\t');
                let a = cols.next().unwrap_or("");
                let d = cols.next().unwrap_or("");
                if a == "-" || d == "-" {
                    continue;
                }
                added += a.parse::<usize>().unwrap_or(0);
                deleted += d.parse::<usize>().unwrap_or(0);
            }
        }
        Some((added, deleted))
    }
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

fn render_widget(name: &str, data: &SessionData, git: &GitContext) -> Option<String> {
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
        "cost-rate" => {
            let ms = data.cost.total_duration_ms;
            if ms == 0 {
                return None;
            }
            let hours = ms as f64 / 3_600_000.0;
            let rate = data.cost.total_cost_usd / hours;
            Some(format!("{} ${rate:.2}/h", label("rate")))
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
        "git-branch" => git.branch().map(|b| format!("{CYAN}{b}{RESET}")),
        "git-ahead" => {
            let (ahead, behind) = git.ahead_behind()?;
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
            let counts = git.file_counts()?;
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
        "git-lines" => {
            let (added, deleted) = git.diff_lines()?;
            if added == 0 && deleted == 0 {
                None
            } else {
                Some(format!("{GREEN}+{added}{RESET}/{RED}-{deleted}{RESET}"))
            }
        }
        "last-commit" => git.last_commit().map(|s| format!("{} {s}", label("last"))),
        "git-status" => {
            let (staged, modified) = git.status_counts()?;
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
    fn format_age_compact_floors_to_minutes() {
        assert_eq!(format_age_compact(0), "0m");
        assert_eq!(format_age_compact(45), "0m");
        assert_eq!(format_age_compact(60), "1m");
        assert_eq!(format_age_compact(12 * 60 + 59), "12m");
        assert_eq!(format_age_compact(60 * 60), "1h 0m");
        assert_eq!(format_age_compact(2 * 3600 + 15 * 60), "2h 15m");
        assert_eq!(format_age_compact(24 * 3600), "1d 0h");
        assert_eq!(format_age_compact(3 * 86400 + 4 * 3600 + 30 * 60), "3d 4h");
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
