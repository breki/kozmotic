# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code)
when working with code in this repository.

## Project Goals

- Implement AI agent-friendly, fast and portable CLI tools
  written in Rust
- Dogfood these tools in the project itself

## Development Practices

- **Domain-Driven Design (DDD)**: Design driven by domain
  concepts
- **Test-Driven Development (TDD)**: Write tests before
  implementation
- **Conventional Commits**: Clear commit messages for clean
  history
- **Ask before assuming**: When in doubt or when multiple
  approaches exist, use the `AskUserQuestion` tool to
  clarify rather than guessing
- **80-column markdown**: Wrap all `.md` files at 80 columns

## Build & Development Commands

Use the wrapper scripts in `scripts/` instead of
calling cargo directly. This makes permission
control easier via `settings.local.json`.

- `bash scripts/validate.sh` — run all checks
  (fmt + clippy + test) in one step
- `bash scripts/build.sh` — build the project
- `bash scripts/run.sh <args>` — run locally
  (e.g., `bash scripts/run.sh agent-ping --sound Stop`)
- `bash scripts/test.sh` — run all tests
- `bash scripts/test.sh <test_name>` — run a single
  test
- `bash scripts/fmt.sh` — auto-format code
- `bash scripts/clippy.sh` — lint (warnings as errors)
- `bash scripts/self-install.sh` — install binary to
  `~/.claude/bin/`

## Architecture

Kozmotic is an early-stage Rust CLI providing
agent-friendly tools with structured JSON output. The
project is a Cargo workspace with two crates:

- **`kozmotic`** (root) — the main CLI binary
- **`xtask`** — dev automation tasks
  (`cargo xtask validate`, etc.)

### CLI structure

Uses a subcommand pattern via `clap` derive.

- `Cli` — top-level parser with a global `--format`
  flag (`json` | `human`)
- `Commands` — enum of subcommands (`Example`,
  `AgentPing`, `StatusLine`, `Self_`)
- `Output<T>` (`src/output.rs`) — generic JSON
  response wrapper with `status`, `data`, and
  `metadata` fields. Use
  `Output::success(tool_name, data)` to construct
  responses.

### Modules

- `src/main.rs` — CLI definition and dispatch
- `src/output.rs` — `Output<T>`, `OutputFormat`
- `src/agent_ping.rs` — sound notification tool
- `src/self_install.rs` — binary installer
- `src/status_line.rs` — status bar formatter

### Adding a new tool

Add a variant to `Commands`, handle it in the
`main()` match, and wrap output with
`Output::success()`. Respect the `--format` flag for
JSON vs human-readable output.

### Integration tests

`tests/integration_test.rs` uses `assert_cmd` to
invoke the binary and `predicates` to validate
stdout. Tests run against the compiled binary, not
library code.

## CI

GitHub Actions runs on all three platforms (ubuntu, windows,
macos). CI enforces:
- `cargo test`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`

Rust edition is 2024. Requires stable toolchain.

## Acceptance Criteria

Before completing any task, run
`bash scripts/validate.sh`, which checks:

1. **Formatting**: `cargo fmt --all -- --check`
2. **No warnings**:
   `cargo clippy --all-targets -- -D warnings`
3. **All tests pass**: `cargo test`
4. **Coverage >= 95%**

## Planning

`TODO.md` tracks ideas and upcoming tasks. Check it
before starting new work and keep it up to date as
items are completed or added.

## Semantic Versioning

Follow [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** — breaking changes to CLI interface or JSON
  output schema
- **MINOR** — new subcommands, flags, or backwards-compatible
  features
- **PATCH** — bug fixes, documentation, internal refactors

The app version lives in `Cargo.toml` and is the **single
source of truth**.

When bumping the version:
1. Update `version` in `Cargo.toml`
2. Add a new section to `CHANGELOG.md`
3. Commit both changes together

## Release Notes

Maintain `CHANGELOG.md` using the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
format. Group changes under these headings:

- **Added** — new features
- **Changed** — changes to existing functionality
- **Fixed** — bug fixes
- **Removed** — removed features

Always keep an `[Unreleased]` section at the top for
in-progress work. When releasing, rename `[Unreleased]`
to `[X.Y.Z] - YYYY-MM-DD` and add a fresh
`[Unreleased]` above it.

## Commit Messages

Use Conventional Commits format with an AI-generated footer.

```
type(scope): subject

Body text here.

AI-Generated: <AgentName> (<ModelName> <Date>)
```

**Header** — `type(scope): description`
- 50 characters max (including type and scope)
- Imperative mood ("add" not "added")
- No period at the end

**Body**
- Wrap at 72 characters
- Explain *what* and *why*, not *how*

**Footer**
- `AI-Generated: Claude Code (<ModelName> <Date>)`
- Omit `Refs:` line if there is no Jira ticket

**Do NOT include:**
- `Co-Authored-By` lines
- `Generated with Claude Code` lines
