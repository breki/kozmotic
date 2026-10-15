//! Codex Stop-hook adapter using the existing Git widgets.
//!
//! Codex consumes the hook JSON directly, so this command deliberately
//! bypasses Kozmotic's general output envelope and format flag.

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use serde::{Deserialize, Serialize};

use super::git::{self, GitContext};
use super::theme::{CYAN, DIM, GREEN, RED, RESET, YELLOW};
use super::widget::Widget;

#[derive(Deserialize)]
struct HookInput {
    cwd: PathBuf,
}

#[derive(Default, Serialize)]
struct HookOutput {
    #[serde(rename = "systemMessage", skip_serializing_if = "Option::is_none")]
    system_message: Option<String>,
}

/// Read the hook payload and emit an informational summary, never a
/// continuation decision. Codex bounds execution through its hook timeout.
pub fn handle() -> ExitCode {
    let mut input = String::new();
    let output = if io::stdin().read_to_string(&mut input).is_ok() {
        summarize(&input)
    } else {
        HookOutput::default()
    };
    serde_json::to_writer(io::stdout().lock(), &output)
        .map_or(ExitCode::FAILURE, |()| ExitCode::SUCCESS)
}

fn summarize(input: &str) -> HookOutput {
    let Ok(payload) = serde_json::from_str::<HookInput>(input) else {
        return HookOutput::default();
    };
    if payload.cwd.as_os_str().is_empty() {
        return HookOutput::default();
    }
    let context = GitContext::new(payload.cwd);
    if !context.is_work_tree() {
        return HookOutput::default();
    }
    let widgets = [
        Widget::GitBranch,
        Widget::GitAhead,
        Widget::GitFiles,
        Widget::GitLines,
        Widget::LastCommit,
    ];
    let rendered = widgets
        .iter()
        .filter_map(|widget| git::render(widget, &context))
        .collect::<Vec<_>>()
        .join(" | ");
    let message = plain_summary(rendered);
    HookOutput {
        system_message: Some(format!("Git: {message}")),
    }
}

/// Only Kozmotic's fixed palette adds escape sequences; widget values
/// already pass through the shared sanitizer before rendering.
fn plain_summary(rendered: String) -> String {
    let plain = [DIM, RESET, CYAN, GREEN, YELLOW, RED]
        .iter()
        .fold(rendered, |text, code| text.replace(code, ""));
    plain
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        .take(995) // Leave five characters for "Git: ".
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_git_status_plain_summary_removes_palette_and_controls() {
        let rendered = format!(
            "{CYAN}main{RESET} | {DIM}git{RESET} {YELLOW}2mod{RESET}\n\u{202e}"
        );
        assert_eq!(plain_summary(rendered), "main | git 2mod");
    }

    #[test]
    fn codex_git_status_plain_summary_bounds_unicode_without_splitting() {
        let result = plain_summary("🦋".repeat(1200));
        assert_eq!(result.chars().count(), 995);
        assert!(result.chars().all(|c| c == '🦋'));
    }
}
