//! The `status-line` subcommand: read Claude Code's session JSON on
//! stdin and render the configured widgets.
//!
//! Widgets are grouped into modules by the data they read — session
//! payload, git, host, process environment, API health — and each
//! module declines widgets it does not own, so [`render_widget`] is a
//! chain of those families.

use std::io::Read;
use std::process::ExitCode;

mod api_status;
mod env_var;
mod format;
mod git;
mod layout;
mod session;
mod system;
mod theme;
mod widget;

use crate::output::{CliError, OutputFormat, Tool, emit_error};
use git::GitContext;
use layout::LineSpec;
use session::SessionData;
use system::SystemContext;
use theme::{RED, RESET};
use widget::Widget;

/// Derives `clap::Args` directly -- see the note on
/// [`crate::sessions::PromptsArgs`].
#[derive(clap::Args)]
pub struct StatusLineArgs {
    /// Widgets to show (comma-separated; ";" splits lines,
    /// "~" right-aligns the rest of a line)
    #[arg(long, default_value = "model,context,cost")]
    pub show: String,

    /// Separator between widgets
    #[arg(long, default_value = " | ")]
    pub separator: String,

    /// Columns to right-align against. Defaults to `COLUMNS`, else
    /// the terminal width, else 80 — see [`layout::resolve_width`].
    #[arg(long)]
    pub width: Option<usize>,
}

/// Why the status line could not be rendered.
///
/// Typed so a hook can tell "nothing arrived on stdin" from "the
/// payload was malformed" by `code`, instead of matching on prose.
#[derive(Debug, thiserror::Error)]
pub enum StatusLineError {
    #[error("no input on stdin")]
    NoInput,
    #[error("invalid JSON: {0}")]
    InvalidJson(String),
    #[error("{0}")]
    UnknownWidget(String),
}

impl CliError for StatusLineError {
    fn code(&self) -> &'static str {
        match self {
            StatusLineError::NoInput => "NO_INPUT",
            StatusLineError::InvalidJson(_) => "INVALID_JSON",
            StatusLineError::UnknownWidget(_) => "UNKNOWN_WIDGET",
        }
    }
}

pub fn handle_status_line(
    format: OutputFormat,
    args: &StatusLineArgs,
) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err()
        || input.trim().is_empty()
    {
        return fail(format, &StatusLineError::NoInput);
    }

    let data: SessionData = match serde_json::from_str(&input) {
        Ok(d) => d,
        Err(e) => {
            return fail(format, &StatusLineError::InvalidJson(e.to_string()));
        }
    };

    let git = GitContext::new(data.working_dir());
    let sys = SystemContext::new(data.working_dir());
    // Resolved once: probing the terminal per line would be wasteful
    // and could report different widths mid-render.
    let width = layout::resolve_width(args.width);

    // Support multi-line: split on ";" to get lines
    for line_spec in args.show.split(';') {
        let spec = match LineSpec::parse(line_spec) {
            Ok(spec) => spec,
            Err(e) => {
                return fail(
                    format,
                    &StatusLineError::UnknownWidget(e.to_string()),
                );
            }
        };
        let render = |widgets: &[Widget]| -> Vec<String> {
            widgets
                .iter()
                .filter_map(|w| render_widget(w, &data, &git, &sys))
                .collect()
        };
        let left = render(&spec.left);
        let right = render(&spec.right);
        if left.is_empty() && right.is_empty() {
            continue;
        }
        println!("{}", layout::compose(&left, &right, &args.separator, width));
    }

    ExitCode::SUCCESS
}

/// Report a failure through the shared envelope *and* leave
/// something visible in the status bar.
///
/// The stdout line is deliberate: a silent failure makes the status
/// line vanish entirely, which is worse than showing what went wrong.
/// The stderr envelope is what a hook or script parses, and it now
/// honours `--format` like every other subcommand.
fn fail(format: OutputFormat, err: &StatusLineError) -> ExitCode {
    println!("{RED}status-line: {err}{RESET}");
    emit_error(format, Tool::StatusLine, err)
}

/// Ask each widget family in turn. Names are disjoint across
/// families, so the first `Some` wins and an unknown name falls
/// through to `None`, which the caller skips.
fn render_widget(
    widget: &Widget,
    data: &SessionData,
    git: &GitContext,
    sys: &SystemContext,
) -> Option<String> {
    session::render(widget, data)
        .or_else(|| git::render(widget, git))
        .or_else(|| system::render(widget, sys))
        .or_else(|| env_var::render(widget))
        // Last: the only family that may touch the network.
        .or_else(|| api_status::render(widget))
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
        // PATH is set on every platform kozmotic targets.
        let env: Widget = "env:PATH".parse().expect("valid widget");
        for widget in [Widget::Cost, Widget::GitFiles, Widget::Ram, env] {
            let out = render_widget(&widget, &data, &git, &sys)
                .unwrap_or_else(|| panic!("{widget} should render"));
            assert!(!out.is_empty(), "{widget}");
        }
    }

    /// Every declared widget must be owned by exactly one family.
    ///
    /// Previously unrepresentable: an unknown name and a widget no
    /// family claimed both produced `None`, so a variant nobody
    /// handled was indistinguishable from a typo. Now that `--show`
    /// only yields real `Widget`s, a variant that renders nothing on
    /// a default context is a genuine gap.
    #[test]
    fn every_widget_is_claimed_by_a_family() {
        let (data, git, sys) = contexts();
        let owners = |w: &Widget| {
            [
                session::render(w, &data).is_some(),
                git::render(w, &git).is_some(),
                system::render(w, &sys).is_some(),
                env_var::render(w).is_some(),
            ]
            .iter()
            .filter(|claimed| **claimed)
            .count()
        };
        // `Widget::ALL` holds only the fixed names, so the env
        // family has to be appended by hand or it goes unchecked —
        // three families absorb it in a `_ => None` arm today.
        let env: Widget = "env:PATH".parse().expect("valid widget");
        for widget in Widget::ALL.iter().chain(std::iter::once(&env)) {
            // `api-status` reaches the network; the rest must be
            // claimed by at most one local family.
            if *widget == Widget::ApiStatus {
                continue;
            }
            assert!(owners(widget) <= 1, "{widget} claimed by two families");
        }
    }
}
