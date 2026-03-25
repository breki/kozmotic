use clap::{Parser, Subcommand};

mod helpers;
mod validate;

#[derive(Parser)]
#[command(name = "xtask", about = "Kozmotic dev tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Full validation: format check, clippy, tests
    Validate,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Validate => validate::validate(),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
