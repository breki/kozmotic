//! The Claude Code session payload and every widget rendered from
//! it: model, context, cost, tokens, timings, rate limits, and the
//! session's own identity fields.

use std::path::PathBuf;

use serde::Deserialize;

use super::format;
use super::theme::{GREEN, RED, RESET, label, usage_color};

#[derive(Deserialize, Default)]
pub struct SessionData {
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

fn deserialize_resets_at<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    match Option::<Value>::deserialize(deserializer)? {
        Some(Value::Number(n)) => Ok(n.as_i64().unwrap_or(0)),
        Some(Value::String(s)) => Ok(format::parse_rfc3339(&s).unwrap_or(0)),
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

impl SessionData {
    /// The directory the session is working in: the workspace path
    /// from the session JSON, falling back to the process's own
    /// directory.
    pub fn working_dir(&self) -> PathBuf {
        if self.workspace.current_dir.is_empty() {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        } else {
            PathBuf::from(&self.workspace.current_dir)
        }
    }
}

fn render_rate_limit(
    lbl: &str,
    bucket: &RateLimitBucket,
    reset_fmt: &str,
) -> Option<String> {
    let pct = bucket.used_percentage;
    let has_reset = bucket.resets_at != 0;
    if pct <= 0.0 && !has_reset {
        return None;
    }
    let mut out = format!("{} {pct:.0}%", label(lbl));
    if let Some(when) = format::reset_time(bucket.resets_at, reset_fmt) {
        use std::fmt::Write as _;
        let _ = write!(out, " (→{when})");
    }
    Some(out)
}

/// Render a session-backed widget, or `None` when the name belongs to
/// another family or the widget has nothing to say.
pub fn render(name: &str, data: &SessionData) -> Option<String> {
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
            let color = usage_color(pct);
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
            Some(format!("{} {}", label("time"), format::duration_ms(ms)))
        }
        "api-duration" => {
            let ms = data.cost.total_api_duration_ms;
            Some(format!("{} {}", label("api"), format::duration_ms(ms)))
        }
        "tokens" => {
            let input = data.context_window.total_input_tokens;
            let output = data.context_window.total_output_tokens;
            Some(format!(
                "{} {} in / {} out",
                label("tok"),
                format::tokens(input),
                format::tokens(output)
            ))
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
            render_rate_limit("5h", &data.rate_limits.five_hour, "%H:%M")
        }
        "rate-limit-7d" => {
            render_rate_limit("7d", &data.rate_limits.seven_day, "%a %H:%M")
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
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_null_resets_at() {
        let json = r#"{"rate_limits":{"five_hour":{"used_percentage":73.2,"resets_at":null}}}"#;
        let data: SessionData =
            serde_json::from_str(json).expect("should parse");
        assert_eq!(data.rate_limits.five_hour.used_percentage, 73.2);
        assert_eq!(data.rate_limits.five_hour.resets_at, 0);
    }

    #[test]
    fn accepts_integer_resets_at() {
        let json = r#"{"rate_limits":{"five_hour":{"used_percentage":51,"resets_at":1776711600}}}"#;
        let data: SessionData =
            serde_json::from_str(json).expect("should parse");
        assert_eq!(data.rate_limits.five_hour.resets_at, 1_776_711_600);
    }

    #[test]
    fn accepts_rfc3339_resets_at() {
        let json = r#"{"rate_limits":{"five_hour":{"resets_at":"2026-04-20T00:00:00Z"}}}"#;
        let data: SessionData =
            serde_json::from_str(json).expect("should parse");
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
        let out =
            render_rate_limit("5h", &bucket, "%H:%M").expect("should render");
        assert!(out.contains("0%"));
        assert!(out.contains("→"));
    }

    #[test]
    fn working_dir_prefers_workspace() {
        let mut data = SessionData::default();
        data.workspace.current_dir = "/tmp/somewhere".to_string();
        assert_eq!(data.working_dir(), PathBuf::from("/tmp/somewhere"));
    }

    #[test]
    fn working_dir_falls_back_to_process_dir() {
        let data = SessionData::default();
        let expected =
            std::env::current_dir().expect("process should have a cwd");
        assert_eq!(data.working_dir(), expected);
    }

    #[test]
    fn empty_session_fields_render_nothing() {
        let data = SessionData::default();
        for widget in
            ["model", "vim", "worktree", "agent", "directory", "session"]
        {
            assert_eq!(render(widget, &data), None, "{widget}");
        }
    }

    #[test]
    fn cost_rate_hidden_until_session_has_duration() {
        let data = SessionData::default();
        assert_eq!(render("cost-rate", &data), None);
    }

    #[test]
    fn foreign_widget_name_is_declined() {
        let data = SessionData::default();
        assert_eq!(render("git-branch", &data), None);
    }
}
