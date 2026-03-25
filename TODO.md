# TODO

- Add clippy lint config to `Cargo.toml`
  (`[lints.rust]` and `[lints.clippy]` sections)
- Add `.cargo/config.toml` with cargo aliases
- Add test profile optimization (`opt-level = 1`)
- Add Cargo metadata (authors, license, repository,
  keywords, categories)
- Add xtask crate with validate command (clippy +
  test + fmt check)
- Add colored error output using the `colored` crate
- Add developer diary (`docs/developer/DIARY.md`)
- Add coverage enforcement with `cargo-llvm-cov`
  (per-module 95% thresholds)
- Update CLAUDE.md with an instruction to use
  AskUserQuestion when in doubt or when there are
  multiple ways of doing something
- Start dog-fooding the tools from this project in
  the agent hooks
- Plan a CLI tool for Claude Code status line that
  allows the user to specify which things they want
  in the status line via command line parameters.
  Investigate what information would be most useful
  and how to get it from the agent's context or
  environment.

## Done

- Make a plan of importing good tools and practices
  from the ledgerstone project (Rust part only)
  (2026-03-25)
- Restructure code into smaller, more manageable
  modules (extract from single-file `src/main.rs`)
  (2026-03-25)
- Add Claude Code Stop hook to run `cargo clippy`
  on modified files and `cargo test` (2026-03-25)
