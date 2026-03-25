# TODO

- Add test profile optimization (`opt-level = 1`)
- Add Cargo metadata (authors, license, repository,
  keywords, categories)
- Add colored error output using the `colored` crate
- Add coverage enforcement with `cargo-llvm-cov`
  (per-module 95% thresholds)
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
- Update CLAUDE.md to reference AskUserQuestion tool
  (2026-03-25)
- Dog-food agent-ping in Stop and Notification hooks
  (2026-03-25)
- Add clippy lint config to Cargo.toml (2026-03-25)
- Add xtask crate with validate command and cargo
  aliases (2026-03-25)
- Add developer diary (2026-03-25)
- Add /sound skill and mute-file support in
  agent-ping (2026-03-25)
- Documented global install: hooks go in
  ~/.claude/settings.json, skills in
  ~/.claude/skills/ (2026-03-25)
- Add wrapper scripts for permission control
  (2026-03-25)
