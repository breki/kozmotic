//! Extracting what the user actually typed out of a session
//! transcript.
//!
//! A transcript is JSON Lines, one record per line, and `user`
//! records cover far more than user input: tool results, replayed
//! system notices, and the local-command plumbing behind slash
//! commands all arrive wearing the same `"type": "user"` label. What
//! separates a real prompt is that its `message.content` is a plain
//! string, it carries no `toolUseResult`, and it is not flagged
//! `isMeta`. The remaining synthetic entries announce themselves with
//! a leading XML-ish tag, which is what [`classify`] keys on.

use std::collections::VecDeque;
use std::io::BufRead;

use serde::{Deserialize, Serialize};

/// Lines longer than this are skipped rather than parsed.
///
/// Transcripts embed tool results, pasted files, and base64 blobs, so
/// a single line can be megabytes. Nothing useful to this command is
/// that long — a prompt that big is not a prompt — and parsing it
/// costs far more than skipping it.
const MAX_LINE_BYTES: usize = 1 << 20;

/// Whether slash-command invocations are part of the result.
///
/// A bare `bool` here reads as `extract(text, true)` at the call
/// site, and the CLI flag is spelled negatively (`--no-commands`),
/// which is exactly where an inversion slips through unnoticed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandFilter {
    Include,
    Omit,
}

/// Tags that wrap content Claude Code generated on the user's
/// behalf. None of it was typed, so none of it is a prompt.
const SYNTHETIC_TAGS: &[&str] = &[
    "<local-command-caveat>",
    "<local-command-stdout>",
    "<task-notification>",
    "<system-reminder>",
    "<user-memory-input>",
];

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Free text the user typed.
    Prompt,
    /// A slash command the user invoked.
    Command,
}

#[derive(Debug, Serialize)]
pub struct Prompt {
    /// 1-based position in the session's user input, counting both
    /// prompts and commands. Stays stable when filters drop entries,
    /// so it identifies a prompt within its session.
    pub index: usize,
    pub kind: Kind,
    /// The prompt text, or the command's arguments when `kind` is
    /// `command` (empty when it was invoked bare).
    pub text: String,
    /// The slash command, including its leading `/`. Only set for
    /// `kind: command`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

/// One line of the transcript. Every field is optional because the
/// file mixes a dozen record shapes and we only read `user` ones.
#[derive(Deserialize)]
struct Record {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default, rename = "isMeta")]
    is_meta: bool,
    #[serde(default, rename = "isSidechain")]
    is_sidechain: bool,
    message: Option<Message>,
    #[serde(rename = "toolUseResult")]
    tool_use_result: Option<serde_json::Value>,
    uuid: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
}

#[derive(Deserialize)]
struct Message {
    content: Option<serde_json::Value>,
}

/// Parse a transcript, keeping only user input.
///
/// Streams line by line and retains at most `limit` prompts, so cost
/// tracks the longest line rather than the file. Transcripts reach
/// hundreds of megabytes on a long session, and the common call is
/// for the last handful of prompts.
///
/// Malformed lines are skipped rather than fatal: a live session is
/// appending to this file, so the last line can be a partial write,
/// and one bad line should not lose the other thousand. Invalid
/// UTF-8 is lossily decoded for the same reason — a torn multi-byte
/// character at EOF damages its own line and nothing else. A genuine
/// I/O failure is still returned as an error.
pub fn extract(
    mut reader: impl BufRead,
    filter: CommandFilter,
    limit: Option<usize>,
) -> std::io::Result<Vec<Prompt>> {
    let mut kept: VecDeque<Prompt> = VecDeque::new();
    let mut raw = Vec::new();
    let mut seen = 0;

    loop {
        raw.clear();
        if reader.read_until(b'\n', &mut raw)? == 0 {
            break;
        }
        if raw.len() > MAX_LINE_BYTES {
            continue;
        }
        let line = String::from_utf8_lossy(&raw);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(record) = serde_json::from_str::<Record>(line) else {
            continue;
        };
        let Some(text) = user_text(&record) else {
            continue;
        };
        let Some((kind, command, body)) = classify(text) else {
            continue;
        };
        // Number before filtering, so an index means the same thing
        // whether or not commands were asked for, and survives the
        // window below.
        seen += 1;
        if kind == Kind::Command && filter == CommandFilter::Omit {
            continue;
        }
        kept.push_back(Prompt {
            index: seen,
            kind,
            text: body,
            command,
            timestamp: record.timestamp,
            uuid: record.uuid,
            git_branch: record.git_branch,
        });
        if let Some(limit) = limit
            && kept.len() > limit
        {
            kept.pop_front();
        }
    }

    Ok(kept.into())
}

/// The string content of a record that could be user input, or
/// `None` for everything else.
///
/// Sidechain records belong to subagents, whose "user" turns are
/// prompts we wrote, not the user.
fn user_text(record: &Record) -> Option<&str> {
    if record.kind.as_deref() != Some("user")
        || record.is_meta
        || record.is_sidechain
        || record.tool_use_result.is_some()
    {
        return None;
    }
    // An array content is a tool_result or a structured block; only a
    // bare string is something the user typed.
    record.message.as_ref()?.content.as_ref()?.as_str()
}

/// Sort user-typed content into a prompt, a slash command, or
/// nothing at all.
fn classify(text: &str) -> Option<(Kind, Option<String>, String)> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if SYNTHETIC_TAGS.iter().any(|tag| trimmed.starts_with(tag)) {
        return None;
    }
    if let Some(name) = tag_value(trimmed, "command-name") {
        let args = tag_value(trimmed, "command-args").unwrap_or_default();
        return Some((Kind::Command, Some(name), args));
    }
    Some((Kind::Prompt, None, trimmed.to_string()))
}

/// The text inside `<tag>…</tag>`, trimmed. Order-independent, since
/// Claude Code emits these tags in varying order.
fn tag_value(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run the extractor over an in-memory transcript. Reading from
    /// a `&[u8]` cannot fail, so the tests assert on the prompts.
    fn run(input: &str, filter: CommandFilter) -> Vec<Prompt> {
        extract(input.as_bytes(), filter, None).expect("in-memory read")
    }

    fn run_limited(input: &str, limit: usize) -> Vec<Prompt> {
        extract(input.as_bytes(), CommandFilter::Include, Some(limit))
            .expect("in-memory read")
    }

    fn user(content: &str) -> String {
        format!(
            r#"{{"type":"user","uuid":"u1","timestamp":"2026-08-14T19:00:00Z",
               "gitBranch":"main","message":{{"role":"user","content":{}}}}}"#,
            serde_json::Value::String(content.to_string())
        )
        .replace('\n', "")
    }

    #[test]
    fn keeps_typed_prompts() {
        let out = run(&user("add a widget"), CommandFilter::Include);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, Kind::Prompt);
        assert_eq!(out[0].text, "add a widget");
        assert_eq!(out[0].index, 1);
        assert_eq!(out[0].git_branch.as_deref(), Some("main"));
        assert_eq!(out[0].uuid.as_deref(), Some("u1"));
        assert!(out[0].command.is_none());
    }

    #[test]
    fn recognises_slash_commands_in_either_tag_order() {
        let a = user(
            "<command-name>/commit</command-name>\n\
             <command-message>commit</command-message>\n\
             <command-args>--amend</command-args>",
        );
        let b = user(
            "<command-message>release</command-message>\n\
             <command-name>/release</command-name>",
        );
        let out = run(&format!("{a}\n{b}"), CommandFilter::Include);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].kind, Kind::Command);
        assert_eq!(out[0].command.as_deref(), Some("/commit"));
        assert_eq!(out[0].text, "--amend");
        assert_eq!(out[1].command.as_deref(), Some("/release"));
        assert_eq!(out[1].text, "");
    }

    #[test]
    fn commands_can_be_excluded() {
        let input = format!(
            "{}\n{}",
            user("<command-name>/commit</command-name>"),
            user("real prompt")
        );
        let out = run(&input, CommandFilter::Omit);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "real prompt");
        // The dropped command still consumed index 1, so this prompt
        // keeps the same index it has in an unfiltered listing.
        assert_eq!(out[0].index, 2);
    }

    #[test]
    fn drops_synthetic_and_non_user_records() {
        let lines = [
            user(
                "<local-command-stdout>Login successful</local-command-stdout>",
            ),
            user("<task-notification>\n<task-id>x</task-id>"),
            user("<system-reminder>be good</system-reminder>"),
            user("<local-command-caveat>Caveat</local-command-caveat>"),
            user("   "),
            r#"{"type":"assistant","message":{"content":"hi"}}"#.to_string(),
            r#"{"type":"last-prompt","lastPrompt":"hello"}"#.to_string(),
            r#"{"type":"user","isMeta":true,"message":{"content":"meta"}}"#
                .to_string(),
            r#"{"type":"user","isSidechain":true,"message":{"content":"sub"}}"#
                .to_string(),
            r#"{"type":"user","toolUseResult":{"x":1},
                "message":{"content":"tool output"}}"#
                .replace('\n', ""),
            r#"{"type":"user","message":{"content":[{"type":"tool_result"}]}}"#
                .to_string(),
            r#"{"type":"user"}"#.to_string(),
            "not json at all".to_string(),
            String::new(),
        ];
        assert!(run(&lines.join("\n"), CommandFilter::Include).is_empty());
    }

    #[test]
    fn numbers_prompts_in_transcript_order() {
        let input = ["one", "two", "three"].map(user).join("\n");
        let out = run(&input, CommandFilter::Include);
        let texts: Vec<_> = out.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["one", "two", "three"]);
        let idx: Vec<_> = out.iter().map(|p| p.index).collect();
        assert_eq!(idx, [1, 2, 3]);
    }

    #[test]
    fn limit_keeps_the_last_prompts_without_renumbering() {
        let input = ["one", "two", "three", "four"].map(user).join("\n");
        let out = run_limited(&input, 2);
        let got: Vec<_> =
            out.iter().map(|p| (p.index, p.text.as_str())).collect();
        assert_eq!(got, [(3, "three"), (4, "four")]);
    }

    #[test]
    fn limit_larger_than_the_transcript_keeps_everything() {
        let input = ["one", "two"].map(user).join("\n");
        assert_eq!(run_limited(&input, 10).len(), 2);
    }

    #[test]
    fn oversized_lines_are_skipped_not_parsed() {
        let huge = user(&"x".repeat(MAX_LINE_BYTES + 1));
        let input = format!("{huge}\n{}", user("kept"));
        let out = run(&input, CommandFilter::Include);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "kept");
        // The skipped line never became a prompt, so it never
        // consumed an index either.
        assert_eq!(out[0].index, 1);
    }

    #[test]
    fn a_torn_multibyte_character_damages_only_its_own_line() {
        // A live session can be mid-write at EOF. Lossy decoding
        // keeps the rest of the transcript readable.
        let mut bytes = user("good one").into_bytes();
        bytes.push(b'\n');
        bytes
            .extend_from_slice(br#"{"type":"user","message":{"content":"bad "#);
        bytes.push(0xff);
        bytes.extend_from_slice(br#""}}"#);
        let out = extract(&bytes[..], CommandFilter::Include, None).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].text, "good one");
        assert!(out[1].text.starts_with("bad "));
    }

    #[test]
    fn unterminated_tag_is_treated_as_prose() {
        let out = run(&user("<command-name>/oops"), CommandFilter::Include);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, Kind::Prompt);
    }
}
