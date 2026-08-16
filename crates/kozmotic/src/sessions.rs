//! The `sessions` subcommand family: query Claude Code's own
//! transcript store on disk.
//!
//! [`store`] finds the transcript, [`prompts`] reads user input out
//! of it, and this module shapes the result for the CLI.

use std::path::PathBuf;
use std::process::ExitCode;

mod prompts;
mod store;

use crate::output::{Output, OutputFormat};
use prompts::Kind;
use store::StoreError;

pub struct PromptsArgs {
    /// Session id to read. Defaults to the current session, then to
    /// the project's most recent one.
    pub session: Option<String>,
    /// Project directory whose sessions to search. Defaults to cwd.
    pub project: Option<PathBuf>,
    /// Keep only the last N prompts.
    pub limit: Option<usize>,
    /// Omit slash-command invocations.
    pub no_commands: bool,
}

#[derive(serde::Serialize)]
struct PromptsData {
    session_id: String,
    project: String,
    transcript: String,
    count: usize,
    prompts: Vec<prompts::Prompt>,
}

pub fn handle_prompts(format: &OutputFormat, args: PromptsArgs) -> ExitCode {
    match gather(args) {
        Ok(data) => {
            emit(format, &data);
            ExitCode::SUCCESS
        }
        Err(err) => emit_error(format, &err),
    }
}

fn gather(args: PromptsArgs) -> Result<PromptsData, StoreError> {
    let root = store::projects_root()?;
    // An explicit --session wins; otherwise inherit the session we
    // are running inside, which is what an agent asking about "my
    // prompts" means. A blank value counts as unspecified rather
    // than as a session whose id is the empty string.
    let session = args
        .session
        .filter(|id| !id.trim().is_empty())
        .or_else(store::current_session_id);
    let transcript = store::resolve(&root, args.project, session)?;

    let text = std::fs::read_to_string(&transcript.path).unwrap_or_default();
    let mut found = prompts::extract(&text, !args.no_commands);

    if let Some(limit) = args.limit
        && found.len() > limit
    {
        // Keep the most recent, but preserve the original numbering
        // so an index still identifies a prompt within the session.
        found.drain(..found.len() - limit);
    }

    Ok(PromptsData {
        session_id: transcript.session_id,
        project: transcript.project_dir.display().to_string(),
        transcript: transcript.path.display().to_string(),
        count: found.len(),
        prompts: found,
    })
}

fn emit(format: &OutputFormat, data: &PromptsData) {
    match format {
        OutputFormat::Json => {
            let output = Output::success("sessions-prompts", data);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            for prompt in &data.prompts {
                let when = prompt
                    .timestamp
                    .as_deref()
                    .map_or("", |t| t.get(..16).unwrap_or(t));
                let what = match (&prompt.kind, &prompt.command) {
                    (Kind::Command, Some(name)) if prompt.text.is_empty() => {
                        name.clone()
                    }
                    (Kind::Command, Some(name)) => {
                        format!("{name} {}", prompt.text)
                    }
                    _ => prompt.text.clone(),
                };
                println!("{:>4}  {when}  {}", prompt.index, first_line(&what));
            }
            if data.prompts.is_empty() {
                println!("No prompts in session {}", data.session_id);
            }
        }
    }
}

/// Human output is one row per prompt, so a multi-line prompt is
/// shown by its first line with a marker for what was elided.
fn first_line(text: &str) -> String {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("");
    let rest = lines.count();
    if rest == 0 {
        first.to_string()
    } else {
        format!("{first} … (+{rest} lines)")
    }
}

fn emit_error(format: &OutputFormat, err: &StoreError) -> ExitCode {
    match format {
        OutputFormat::Json => {
            let output =
                Output::error("sessions-prompts", err.code(), &err.to_string());
            eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => eprintln!("Error [{}]: {}", err.code(), err),
    }
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_text_is_unchanged() {
        assert_eq!(first_line("just this"), "just this");
        assert_eq!(first_line(""), "");
    }

    #[test]
    fn multi_line_text_reports_what_was_elided() {
        assert_eq!(first_line("a\nb\nc"), "a … (+2 lines)");
    }
}
