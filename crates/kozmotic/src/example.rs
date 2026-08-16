//! The `example` subcommand: the reference tool new subcommands are
//! copied from.
//!
//! It lives in its own module for exactly that reason. Implemented
//! inline in `main`, it modelled the one shape no other subcommand
//! uses, so anyone following the "Adding a new tool" recipe in
//! CLAUDE.md started from the wrong pattern.

use std::process::ExitCode;

use crate::output::{OutputFormat, Tool, emit_success};

#[derive(clap::Args)]
pub struct ExampleArgs {
    /// Example argument
    #[arg(short, long)]
    pub name: Option<String>,
}

pub fn handle_example(format: OutputFormat, args: ExampleArgs) -> ExitCode {
    let name = args.name.unwrap_or_else(|| "World".to_string());
    let greeting = format!("Hello, {name}!");

    match format {
        OutputFormat::Json => {
            emit_success(
                format,
                Tool::Example,
                serde_json::json!({ "message": greeting }),
            );
        }
        OutputFormat::Human => println!("{greeting}"),
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_world() {
        assert_eq!(
            handle_example(OutputFormat::Human, ExampleArgs { name: None }),
            ExitCode::SUCCESS
        );
    }
}
