use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::process::ExitCode;

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

#[derive(Clone, Debug)]
enum OutputFormat {
    Json,
    Human,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "human" => Ok(OutputFormat::Human),
            _ => Err(format!("Invalid format: {}. Use 'json' or 'human'", s)),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Example command - will be replaced with actual tools
    Example {
        /// Example argument
        #[arg(short, long)]
        name: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
struct Output<T> {
    status: String,
    data: T,
    metadata: Metadata,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    timestamp: String,
    tool: String,
    version: String,
}

impl<T> Output<T> {
    fn success(tool: &str, data: T) -> Self {
        Self {
            status: "success".to_string(),
            data,
            metadata: Metadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                tool: tool.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
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
                    let output = Output::success("example", data);
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => {
                    println!("Hello, {}!", name);
                }
            }
            ExitCode::SUCCESS
        }
        None => {
            println!("No command specified. Use --help for usage information.");
            ExitCode::FAILURE
        }
    }
}
