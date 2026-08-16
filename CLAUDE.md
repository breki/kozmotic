# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code)
when working with code in this repository.

**IMPORTANT: The working directory is already set to the
project root. NEVER use `cd` to the project root or
`git -C <dir>` -- blanket permission rules cannot be
set for commands starting with `cd` or `git -C`, so
they require manual approval every time.**

## Project Overview

Kozmotic is a portable CLI toolkit for AI agents,
written in Rust. Tools emit structured JSON (or human
output via `--format human`) so they compose cleanly
inside Claude Code hooks, status lines, and slash
commands.

- **Stack**: Rust (edition 2024, stable)
- **Target platforms**: Windows, Linux, macOS
- **Output**: structured JSON envelope via `Output<T>`

### Subcommands

| Command | Purpose |
|---------|---------|
| `kozmotic example` | Reference subcommand for new tools |
| `kozmotic agent-ping` | Play notification sounds (presets, files, tones) |
| `kozmotic status-line` | Format Claude Code session JSON for the status bar |
| `kozmotic sessions prompts` | List a session's user prompts from the transcript store |
| `kozmotic self install` | Install the binary into `~/.claude/bin/` |

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `crates/kozmotic` | CLI binary (the toolkit) |
| `xtask` | Build automation |

## Build Commands

```bash
cargo xtask check             # fast compile check
cargo xtask validate          # fmt + clippy + tests + coverage
cargo xtask test [filter]     # tests only
cargo xtask clippy            # lint only
cargo xtask coverage          # coverage only (>=90%)
cargo xtask fmt               # format code
cargo xtask dupes             # code duplication check
```

Never use raw `cargo test` or `cargo clippy` -- always
go through `xtask`.

### PowerShell Build Script

```powershell
.\build.ps1 validate    # cargo xtask validate
.\build.ps1 test        # tests only
.\build.ps1 build       # validate + release build
.\build.ps1 clean       # clean artifacts
```

## Coding Standards

- Rust edition 2024
- `#[deny(warnings)]` and `#[forbid(unsafe_code)]` via
  workspace lints
- Clippy pedantic where practical (allow-list in
  workspace `Cargo.toml`)
- Error handling: `thiserror` for typed errors;
  `anyhow` is fine in main if/when added
- Wrap markdown at 80 characters per line
- Maximum code line width: 80 characters (`rustfmt.toml`)

## Development Practices

- **Domain-Driven Design (DDD)**: model around domain
  concepts, not framework primitives.
- **Test-Driven Development (TDD)**: write a failing
  test first; make it pass with the smallest code; then
  refactor. Run `cargo xtask test` after each step.
- **Ask before assuming**: when multiple approaches
  exist, use the `AskUserQuestion` tool to clarify
  rather than guessing.

## Adding a new tool

1. Add a variant to `Commands` in
   `crates/kozmotic/src/main.rs`.
2. Implement the handler in its own module under
   `crates/kozmotic/src/`.
3. Wrap the result with `Output::success(tool, data)`.
4. Respect the `--format` flag for JSON vs human
   output.
5. Cover the new path in
   `crates/kozmotic/tests/integration_test.rs` using
   `assert_cmd`.

## Commits

**All commits must go through the `/commit` skill.**
Never use `git commit` directly. No `Co-Authored-By`,
no emoji.

Conventional Commits format with an AI-generated footer:

```
type(scope): subject

Body explaining what and why (wrapped at 72).

AI-Generated: Claude Code (<ModelName> <Date>)
```

- Header: 50 chars max, imperative mood, no trailing
  period.
- Body: 72-char wrap, focuses on what/why.
- Footer: `AI-Generated: Claude Code (<ModelName>
  <Date>)`. No `Co-Authored-By`. No
  `Generated with Claude Code` lines.

## Acceptance Criteria

Before completing any task, run `cargo xtask validate`,
which checks:

1. **Formatting**: `cargo fmt --all -- --check`
2. **No warnings**:
   `cargo clippy --all-targets -- -D warnings`
3. **All tests pass**: `cargo test`
4. **Coverage >= 90%** (per-module floor 85%)

## Semantic Versioning

Follow [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** -- breaking changes to CLI interface or
  JSON output schema
- **MINOR** -- new subcommands, flags, or backwards-
  compatible features
- **PATCH** -- bug fixes, documentation, internal
  refactors

The version lives in `crates/kozmotic/Cargo.toml` and
is the **single source of truth**.

## Release Notes

Maintain `CHANGELOG.md` using the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
format. Group changes under: **Added**, **Changed**,
**Fixed**, **Removed**.

Always keep an `[Unreleased]` section at the top.

## Planning

`TODO.md` tracks upcoming tasks. Check it before
starting new work and keep it up to date as items are
completed or added.

## Skills

| Skill | Purpose |
|-------|---------|
| `/check` | Fast compilation check (no tests) |
| `/test` | Run tests with agent-friendly output |
| `/validate` | Full quality pipeline with stepwise progress |
| `/commit` | Commit with versioning, diary, and conventions |
| `/release` | Prepare a versioned release |
| `/todo` | Add a TODO item or implement the next pending one |
| `/simplify` | Review changed code for quality |
| `/architect` | Project overview and architecture guide |
| `/agent-cli` | Patterns for agent-friendly CLI subcommands |
| `/sound` | Toggle hook sounds on/off |
| `/statusline-setup` | Configure status line widgets |
| `/template-improve` | Log feedback for the rustbase template |
| `/template-sync` | Sync upstream template changes |

## Template Sync

This project tracks its template origin in
`.template-sync.toml`. Use `/template-sync` to pull
improvements from the upstream
[rustbase](https://github.com/breki/rustbase) template.
The command fetches upstream changes, categorizes
them, and helps you selectively apply relevant updates
while preserving your project's customizations.

## Template Feedback

This project was generated from the
[rustbase](https://github.com/breki/rustbase) template.
When you notice anything in the template-provided
files that is suboptimal, incorrect, outdated, or
could be improved, log it in
`docs/developer/template-feedback.md`.
