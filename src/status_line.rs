use std::io::Read;
use std::process::ExitCode;

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
}

#[derive(Deserialize, Default)]
struct CostData {
    #[serde(default)]
    total_cost_usd: f64,
    #[serde(default)]
    total_lines_added: u64,
    #[serde(default)]
    total_lines_removed: u64,
}

#[derive(Deserialize, Default)]
struct RateLimitsData {
    #[serde(default)]
    five_hour: RateLimitBucket,
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

    let widgets: Vec<&str> = args.show.split(',').map(|s| s.trim()).collect();
    let parts: Vec<String> = widgets
        .iter()
        .filter_map(|w| render_widget(w, &data))
        .collect();

    if parts.is_empty() {
        return ExitCode::SUCCESS;
    }

    println!("{}", parts.join(&args.separator));
    ExitCode::SUCCESS
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
                "\x1b[31m" // red
            } else if pct >= 50.0 {
                "\x1b[33m" // yellow
            } else {
                "\x1b[32m" // green
            };
            Some(format!("{color}{pct:.1}%\x1b[0m"))
        }
        "cost" => {
            let cost = data.cost.total_cost_usd;
            Some(format!("${cost:.2}"))
        }
        "lines" => {
            let added = data.cost.total_lines_added;
            let removed = data.cost.total_lines_removed;
            Some(format!("+{added}/-{removed}"))
        }
        "rate-limit" => {
            let pct = data.rate_limits.five_hour.used_percentage;
            if pct > 0.0 {
                Some(format!("rate {pct:.1}%"))
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
        _ => None,
    }
}
