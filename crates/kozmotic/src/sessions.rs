//! The `sessions` subcommand family: query Claude Code's own
//! transcript store on disk.
//!
//! [`store`] finds the transcript, [`prompts`] reads user input out
//! of it, and this module shapes the result for the CLI.

use std::path::PathBuf;
use std::process::ExitCode;

mod prompts;
mod store;

use crate::output::{OutputFormat, Tool, emit_error, emit_success};
use prompts::Kind;
use store::StoreError;

/// Derives `clap::Args` directly rather than mirroring a separate
/// struct in `main`: this is a binary crate, so there is no public
/// API for clap to leak into, and a field-by-field copy between two
/// identical structs type-checks even when a value lands in the
/// wrong slot.
#[derive(clap::Args)]
pub struct PromptsArgs {
    /// Session id to read (default: the current session, else the
    /// project's most recent one)
    #[arg(long)]
    pub session: Option<String>,

    /// Project directory whose sessions to search (default: cwd)
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// Show only the last N prompts
    #[arg(long)]
    pub limit: Option<usize>,

    /// Omit slash-command invocations
    #[arg(long)]
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

pub fn handle_prompts(format: OutputFormat, args: PromptsArgs) -> ExitCode {
    match gather(args) {
        Ok(data) => {
            emit(format, &data);
            ExitCode::SUCCESS
        }
        Err(err) => emit_error(format, Tool::SessionsPrompts, &err),
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

    let filter = if args.no_commands {
        prompts::CommandFilter::Omit
    } else {
        prompts::CommandFilter::Include
    };
    // `--limit` is applied while streaming, keeping the most recent
    // prompts but preserving their original numbering, so an index
    // still identifies a prompt within the whole session.
    let unreadable =
        |e: std::io::Error| StoreError::Unreadable(transcript.path.clone(), e);
    let file = std::fs::File::open(&transcript.path).map_err(unreadable)?;
    let found =
        prompts::extract(std::io::BufReader::new(file), filter, args.limit)
            .map_err(unreadable)?;

    Ok(PromptsData {
        session_id: transcript.session_id,
        project: transcript.project_dir.display().to_string(),
        transcript: transcript.path.display().to_string(),
        count: found.len(),
        prompts: found,
    })
}

fn emit(format: OutputFormat, data: &PromptsData) {
    match format {
        OutputFormat::Json => {
            emit_success(format, Tool::SessionsPrompts, data);
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
