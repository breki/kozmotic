//! The JSON envelope every subcommand emits, and the shared error
//! reporting built on it.
//!
//! The envelope is the tool's public contract — a change to its shape
//! is a MAJOR version bump — so both halves of it are typed here
//! rather than assembled ad hoc at each call site.

use std::process::ExitCode;

use serde::{Deserialize, Serialize};

/// Which subcommand produced an envelope.
///
/// The `tool` field is part of the output schema, so it is an enum
/// rather than a string literal repeated at each call site: a typo in
/// one of several `"agent-ping"` spellings would otherwise produce a
/// subtly wrong envelope that only an exact-match test would catch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Example,
    AgentPing,
    StatusLine,
    SelfInstall,
    SessionsPrompts,
}

impl Tool {
    pub fn as_str(self) -> &'static str {
        match self {
            Tool::Example => "example",
            Tool::AgentPing => "agent-ping",
            Tool::StatusLine => "status-line",
            Tool::SelfInstall => "self-install",
            Tool::SessionsPrompts => "sessions-prompts",
        }
    }
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Human,
}

/// The payload of an error envelope.
///
/// A struct rather than a `json!` literal: this is the shape every
/// hook consumer parses, so it should be reviewable, diffable, and
/// impossible to misspell.
#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorData {
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output<T> {
    status: String,
    data: T,
    metadata: Metadata,
}

#[derive(Serialize, Deserialize, Debug)]
struct Metadata {
    timestamp: String,
    tool: String,
    version: String,
}

impl Metadata {
    fn new(tool: Tool) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool: tool.as_str().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl<T> Output<T> {
    pub fn success(tool: Tool, data: T) -> Self {
        Self {
            status: "success".to_string(),
            data,
            metadata: Metadata::new(tool),
        }
    }

    /// The payload, so tests can assert against a typed value rather
    /// than re-parsing stdout as an untyped `Value`. Test-only: this
    /// is a binary crate, so nothing else reads an envelope back.
    #[cfg(test)]
    pub fn data(&self) -> &T {
        &self.data
    }
}

impl Output<ErrorData> {
    pub fn error(tool: Tool, code: &str, message: &str) -> Self {
        Self {
            status: "error".to_string(),
            data: ErrorData {
                code: code.to_string(),
                message: message.to_string(),
            },
            metadata: Metadata::new(tool),
        }
    }
}

/// An error a subcommand can report through the envelope.
///
/// Every subcommand had grown its own byte-for-byte copy of the
/// reporting code, so the contract — which stream, pretty-printed or
/// not, the `Error [CODE]: msg` human shape — was defined in as many
/// places as there were tools, and drift in any one of them would
/// only surface when a hook consumer broke.
pub trait CliError: std::error::Error {
    /// Stable, machine-readable identifier for this failure.
    fn code(&self) -> &'static str;

    /// Process exit code. Defaults to 1; override to distinguish
    /// classes of failure.
    fn exit_code(&self) -> u8 {
        1
    }
}

/// Print `data` as the tool's success envelope.
pub fn emit_success<T: Serialize>(format: OutputFormat, tool: Tool, data: T) {
    match format {
        OutputFormat::Json => {
            let output = Output::success(tool, data);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {}
    }
}

/// Report `err` on stderr in the requested format and return the
/// failure exit code, so handlers can `return emit_error(..)`.
pub fn emit_error<E: CliError + ?Sized>(
    format: OutputFormat,
    tool: Tool,
    err: &E,
) -> ExitCode {
    match format {
        OutputFormat::Json => {
            let output = Output::error(tool, err.code(), &err.to_string());
            eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            eprintln!("Error [{}]: {}", err.code(), err);
        }
    }
    ExitCode::from(err.exit_code())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("something went wrong")]
    struct Boom;

    impl CliError for Boom {
        fn code(&self) -> &'static str {
            "BOOM"
        }
        fn exit_code(&self) -> u8 {
            2
        }
    }

    #[test]
    fn tool_names_are_the_cli_spelling() {
        assert_eq!(Tool::AgentPing.as_str(), "agent-ping");
        assert_eq!(Tool::SessionsPrompts.to_string(), "sessions-prompts");
    }

    #[test]
    fn success_envelope_carries_the_payload() {
        let out = Output::success(Tool::Example, 42);
        assert_eq!(*out.data(), 42);
        let json = serde_json::to_value(&out).unwrap();
        assert_eq!(json["status"], "success");
        assert_eq!(json["metadata"]["tool"], "example");
    }

    #[test]
    fn error_envelope_is_typed() {
        let out = Output::error(Tool::SelfInstall, "NOPE", "no");
        assert_eq!(out.data().code, "NOPE");
        assert_eq!(out.data().message, "no");
        let json = serde_json::to_value(&out).unwrap();
        assert_eq!(json["status"], "error");
        assert_eq!(json["data"]["code"], "NOPE");
    }

    #[test]
    fn exit_code_defaults_to_one_and_can_be_overridden() {
        #[derive(Debug, thiserror::Error)]
        #[error("plain")]
        struct Plain;
        impl CliError for Plain {
            fn code(&self) -> &'static str {
                "PLAIN"
            }
        }
        assert_eq!(Plain.exit_code(), 1);
        assert_eq!(Boom.exit_code(), 2);
    }
}
