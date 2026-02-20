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
  approaches exist, ask interactively rather than guessing
- **80-column markdown**: Wrap all `.md` files at 80 columns

## Build & Development Commands

- `cargo build` — build the project
- `cargo run -- <args>` — run locally
  (e.g., `cargo run -- example --name Foo`)
- `cargo test` — run all tests (unit + integration)
- `cargo test <test_name>` — run a single test
  (e.g., `cargo test test_example_json_output`)
- `cargo fmt --all -- --check` — check formatting
- `cargo fmt` — auto-format code
- `cargo clippy --all-targets -- -D warnings` — lint
  (treats warnings as errors)

## Architecture

Kozmotic is an early-stage Rust CLI providing agent-friendly
tools with structured JSON output. It uses a subcommand
pattern via `clap` derive.

**Core structure (`src/main.rs`):**
- `Cli` — top-level parser with a global `--format` flag
  (`json` | `human`)
- `Commands` — enum of subcommands (currently just
  `Example`; new tools go here)
- `Output<T>` — generic JSON response wrapper with `status`,
  `data`, and `metadata` fields. Use
  `Output::success(tool_name, data)` to construct responses.

**Adding a new tool:** Add a variant to `Commands`, handle it
in the `main()` match, and wrap output with
`Output::success()`. Respect the `--format` flag for JSON vs
human-readable output.

**Integration tests (`tests/integration_test.rs`):** Use
`assert_cmd` to invoke the binary and `predicates` to
validate stdout. Tests run against the compiled binary, not
library code.

## CI

GitHub Actions runs on all three platforms (ubuntu, windows,
macos). CI enforces:
- `cargo test`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`

Rust edition is 2024. Requires stable toolchain.

## Acceptance Criteria

Before completing any task:

1. **All tests pass**: `cargo test`
2. **Coverage >= 95%**
3. **No warnings**:
   `cargo clippy --all-targets -- -D warnings`

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
