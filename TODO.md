# TODO

## `status-line` subcommand

A new subcommand that reads Claude Code session
JSON from stdin and outputs a formatted status line.
Configured as the `command` in `settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/bin/kozmotic status-line --show model,context,cost"
  }
}
```

### Available data from stdin JSON

- `model.display_name` — current model
- `context_window.used_percentage` — context %
- `cost.total_cost_usd` — session cost
- `cost.total_lines_added/removed` — code changes
- `rate_limits.five_hour.used_percentage` — rate
  limit usage
- `vim.mode` — NORMAL/INSERT
- `agent.name` — agent name

### Implementation plan

- [x] Add `StatusLine` variant to `Commands`
- [x] Parse stdin JSON (serde deserialize, only
  the fields we need)
- [x] Define "widgets": model, context, cost, lines
- [x] `--show` flag: comma-separated widget list
  (default: `model,context,cost`)
- [x] `--separator` flag (default: ` | `)
- [x] Support ANSI colors for context % thresholds
  (green < 50%, yellow < 80%, red >= 80%)
- [x] Output a single line to stdout
- [x] Add integration tests with sample JSON input
- [x] Add rate-limit and vim-mode widgets
- [x] Add a `/statusline-setup` skill that
  configures settings.json for the user
- [x] Document in CLAUDE.md and architect skill
- [x] Add widgets: duration, api-duration, tokens,
  git-branch, git-status, directory, session,
  rate-limit-7d, worktree, agent
- [x] Add multi-line support (semicolon separator)

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
- Plan status-line subcommand (2026-03-25)
