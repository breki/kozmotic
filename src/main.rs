mod agent_ping;
mod output;
mod self_install;
mod status_line;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use agent_ping::{AgentPingArgs, handle_agent_ping};
use output::OutputFormat;
use self_install::handle_self_install;
use status_line::{StatusLineArgs, handle_status_line};

#[derive(Parser)]
#[command(name = "kozmotic")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Output format (json or human)
    #[arg(short, long, value_name = "FORMAT", default_value = "json")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Example command - will be replaced with actual tools
    Example {
        /// Example argument
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Manage the kozmotic installation
    #[command(name = "self", subcommand)]
    Self_(SelfCommands),
    /// Format Claude Code session data for the status line
    #[command(name = "status-line")]
    StatusLine {
        /// Widgets to show (comma-separated)
        #[arg(long, default_value = "model,context,cost")]
        show: String,

        /// Separator between widgets
        #[arg(long, default_value = " | ")]
        separator: String,
    },
    /// Play a notification sound (for hooks and alerts)
    #[command(name = "agent-ping")]
    AgentPing {
        /// Play a built-in preset (hook event name)
        #[arg(long, group = "source")]
        sound: Option<String>,

        /// Play a custom audio file
        #[arg(long, group = "source")]
        file: Option<String>,

        /// Play a generated tone at frequency (Hz)
        #[arg(long, group = "source")]
        frequency: Option<f32>,

        /// Tone duration in ms (--frequency only)
        #[arg(long, default_value = "200")]
        duration: u64,

        /// Volume 0.0-1.0
        #[arg(long, default_value = "0.5")]
        volume: f32,

        /// Play N times
        #[arg(long, default_value = "1")]
        repeat: u32,

        /// Gap between repeats in ms
        #[arg(long, default_value = "100")]
        interval: u64,

        /// List available presets
        #[arg(long)]
        list: bool,

        /// Report what would play, no sound
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum SelfCommands {
    /// Install kozmotic to ~/.claude/bin/
    Install(SelfInstallArgs),
}

#[derive(Args)]
struct SelfInstallArgs {
    /// Override the install directory
    #[arg(long)]
    target_dir: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Example { name }) => {
            let name = name.unwrap_or_else(|| "World".to_string());
            let data = serde_json::json!({
                "message": format!("Hello, {}!", name)
            });

            match cli.format {
                OutputFormat::Json => {
                    let output = output::Output::success("example", data);
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => {
                    println!("Hello, {}!", name);
                }
            }
            ExitCode::SUCCESS
        }
        Some(Commands::StatusLine { show, separator }) => {
            handle_status_line(StatusLineArgs { show, separator })
        }
        Some(Commands::Self_(SelfCommands::Install(args))) => {
            handle_self_install(&cli.format, args.target_dir)
        }
        Some(Commands::AgentPing {
            sound,
            file,
            frequency,
            duration,
            volume,
            repeat,
            interval,
            list,
            dry_run,
        }) => handle_agent_ping(
            &cli.format,
            AgentPingArgs {
                sound,
                file,
                frequency,
                duration,
                volume,
                repeat,
                interval,
                list,
                dry_run,
            },
        ),
        None => {
            println!(
                "No command specified. \
                 Use --help for usage information."
            );
            ExitCode::FAILURE
        }
    }
}
