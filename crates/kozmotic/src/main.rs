mod agent_ping;
mod example;
mod output;
mod self_install;
mod sessions;
mod status_line;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use agent_ping::{AgentPingArgs, handle_agent_ping};
use example::{ExampleArgs, handle_example};
use output::OutputFormat;
use self_install::{SelfInstallArgs, handle_self_install};
use sessions::{PromptsArgs, handle_prompts};
use status_line::{StatusLineArgs, handle_status_line};

#[derive(Parser)]
#[command(name = "kozmotic")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Json)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Example command - the reference tool for new subcommands
    Example(ExampleArgs),
    /// Manage the kozmotic installation
    #[command(name = "self", subcommand)]
    Self_(SelfCommands),
    /// Query Claude Code's session transcripts on disk
    #[command(subcommand)]
    Sessions(SessionCommands),
    /// Format Claude Code session data for the status line
    #[command(name = "status-line")]
    StatusLine(StatusLineArgs),
    /// Play a notification sound (for hooks and alerts)
    #[command(name = "agent-ping")]
    AgentPing(AgentPingArgs),
}

#[derive(Subcommand)]
enum SelfCommands {
    /// Install kozmotic to ~/.claude/bin/
    Install(SelfInstallArgs),
}

#[derive(Subcommand)]
enum SessionCommands {
    /// List the prompts the user sent in a session
    Prompts(PromptsArgs),
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Example(args)) => handle_example(cli.format, args),
        Some(Commands::StatusLine(args)) => {
            handle_status_line(cli.format, &args)
        }
        Some(Commands::Self_(SelfCommands::Install(args))) => {
            handle_self_install(cli.format, args)
        }
        Some(Commands::Sessions(SessionCommands::Prompts(args))) => {
            handle_prompts(cli.format, args)
        }
        Some(Commands::AgentPing(args)) => handle_agent_ping(cli.format, args),
        None => {
            println!(
                "No command specified. \
                 Use --help for usage information."
            );
            ExitCode::FAILURE
        }
    }
}
