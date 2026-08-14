//! The `status-line` subcommand: read Claude Code's session JSON on
//! stdin and render the configured widgets.
//!
//! Widgets are grouped into modules by the data they read — session
//! payload, git, host, API health — and each module declines names it
//! does not own, so [`render_widget`] is a chain of those families.

use std::io::Read;
use std::process::ExitCode;

mod api_status;
mod format;
mod git;
mod session;
mod system;
mod theme;

use git::GitContext;
use session::SessionData;
use system::SystemContext;
use theme::{RED, RESET};

pub struct StatusLineArgs {
    pub show: String,
    pub separator: String,
}

pub fn handle_status_line(args: &StatusLineArgs) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err()
        || input.trim().is_empty()
    {
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
    let sys = SystemContext::new(data.working_dir());
    // Support multi-line: split on ";" to get lines
    let lines: Vec<&str> = args.show.split(';').collect();
    for line_spec in &lines {
        let widgets: Vec<&str> = line_spec.split(',').map(str::trim).collect();
        let parts: Vec<String> = widgets
            .iter()
            .filter_map(|w| render_widget(w, &data, &git, &sys))
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

/// Ask each widget family in turn. Names are disjoint across
/// families, so the first `Some` wins and an unknown name falls
/// through to `None`, which the caller skips.
fn render_widget(
    name: &str,
    data: &SessionData,
    git: &GitContext,
    sys: &SystemContext,
) -> Option<String> {
    session::render(name, data)
        .or_else(|| git::render(name, git))
        .or_else(|| system::render(name, sys))
        .or_else(|| api_status::render(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn contexts() -> (SessionData, GitContext, SystemContext) {
        (
            SessionData::default(),
            GitContext::default(),
            SystemContext::new(PathBuf::from(".")),
        )
    }

    #[test]
    fn dispatch_reaches_each_family() {
        let (data, git, sys) = contexts();
        // One widget per family that always renders on any machine.
        for widget in ["cost", "git-files", "ram"] {
            let out = render_widget(widget, &data, &git, &sys)
                .unwrap_or_else(|| panic!("{widget} should render"));
            assert!(!out.is_empty(), "{widget}");
        }
    }

    #[test]
    fn unknown_widget_renders_nothing() {
        let (data, git, sys) = contexts();
        assert_eq!(render_widget("nope", &data, &git, &sys), None);
    }
}
