/# TODO

- Introduce semantic versioning, release notes and Github releases. 
- Split the status line widgets into their own modules. 
- Provide a CLI switch for right-aligning certain status line widgets.
- Update README with new features and installation instructions.

## Done

- Add api-status widget with cached HTTP fetch
  from status.claude.com (2 min TTL) (2026-03-27)
- Install /statusline-setup and /sound skills
  globally at ~/.claude/skills/ (2026-03-26)
- Add dim labels and color to status line widgets
  (2026-03-26)

- Implement `status-line` subcommand with 16
  widgets, multi-line support, ANSI colors, and
  `/statusline-setup` skill (2026-03-26)
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
- Add test profile optimization (2026-03-25)
- Update CLAUDE.md to use wrapper scripts
  (2026-03-25)
- Add Cargo metadata (2026-03-25)
- Colored error output — skipped, unnecessary
  for agent-first CLI tools (2026-03-25)
- Add coverage reporting to xtask validate
  (no threshold yet, 57%) (2026-03-25)
- Dependency audit: removed unused `anyhow`, all
  remaining deps justified (2026-03-25)
