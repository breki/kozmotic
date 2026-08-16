mod changelog;
mod check;
mod clippy_cmd;
mod coverage;
mod dupes;
mod fmt_cmd;
mod helpers;
mod test_cmd;
mod todo;
mod validate;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Kozmotic dev tasks")]
struct Cli {
    #[command(subcommand)]
    command: XCommand,
}

#[derive(Subcommand)]
enum XCommand {
    /// Fast compilation check (no tests)
    Check,
    /// Run clippy (deny warnings)
    Clippy,
    /// Run all tests
    Test {
        /// Optional test filter
        filter: Option<String>,
        /// Show raw cargo test output
        #[arg(long)]
        verbose: bool,
        /// Run `#[ignore]`-tagged tests instead of the default set
        #[arg(long)]
        ignored: bool,
    },
    /// Run fmt + clippy + tests + coverage
    Validate,
    /// Format code
    Fmt,
    /// Run coverage check (requires cargo-llvm-cov)
    Coverage,
    /// Run code duplication check (requires code-dupes)
    Dupes,
    /// Mechanically edit `CHANGELOG.md` (used by `/commit`) --
    /// insert a bullet under the right `[Unreleased]` subsection
    Changelog {
        #[command(subcommand)]
        action: changelog::ChangelogAction,
    },
    /// Mechanically read/edit `TODO.md` (used by `/todo`) --
    /// list, add, or complete a captured item
    Todo {
        #[command(subcommand)]
        action: todo::TodoAction,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        XCommand::Check => check::check(),
        XCommand::Clippy => clippy_cmd::clippy(),
        XCommand::Test {
            filter,
            verbose,
            ignored,
        } => test_cmd::test(test_cmd::TestOptions {
            filter: filter.as_deref(),
            verbose,
            ignored,
        }),
        XCommand::Validate => validate::validate(),
        XCommand::Fmt => fmt_cmd::fmt(),
        XCommand::Coverage => coverage::coverage(),
        XCommand::Dupes => dupes::dupes(),
        XCommand::Changelog { action } => changelog::changelog(action),
        XCommand::Todo { action } => todo::todo(action),
    };

    if let Err(e) = result {
        eprintln!("xtask error: {e}");
        std::process::exit(1);
    }
}
