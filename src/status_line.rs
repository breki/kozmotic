use std::io::Read;
use std::process::{Command, ExitCode};

use serde::Deserialize;

#[derive(Deserialize, Default)]
struct SessionData {
    #[serde(default)]
    model: ModelData,
    #[serde(default)]
    context_window: ContextData,
    #[serde(default)]
    cost: CostData,
    #[serde(default)]
    rate_limits: RateLimitsData,
    #[serde(default)]
    vim: VimData,
    #[serde(default)]
    workspace: WorkspaceData,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    agent: AgentData,
    #[serde(default)]
    worktree: WorktreeData,
}

#[derive(Deserialize, Default)]
struct ModelData {
    #[serde(default)]
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
}

#[derive(Deserialize, Default)]
struct VimData {
    #[serde(default)]
    mode: String,
}

#[derive(Deserialize, Default)]
struct WorkspaceData {
    #[serde(default)]
    current_dir: String,
}

#[derive(Deserialize, Default)]
struct AgentData {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Default)]
struct WorktreeData {
    #[serde(default)]
    name: String,
}

pub struct StatusLineArgs {
    pub show: String,
    pub separator: String,
}

pub fn handle_status_line(args: StatusLineArgs) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() || input.trim().is_empty() {
        eprintln!("Error: no input on stdin");
        return ExitCode::FAILURE;
    }

    let data: SessionData = match serde_json::from_str(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: invalid JSON: {e}");
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

fn format_duration_ms(ms: u64) -> String {
    let secs = ms / 1000;
    let mins = secs / 60;
    let secs = secs % 60;
    format!("{mins}m {secs}s")
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

const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

fn label(name: &str) -> String {
    format!("{DIM}{name}{RESET}")
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
            let pct = data.rate_limits.five_hour.used_percentage;
            if pct > 0.0 {
                Some(format!("{} {pct:.0}%", label("5h")))
            } else {
                None
            }
        }
        "rate-limit-7d" => {
            let pct = data.rate_limits.seven_day.used_percentage;
            if pct > 0.0 {
                Some(format!("{} {pct:.0}%", label("7d")))
            } else {
                None
            }
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
