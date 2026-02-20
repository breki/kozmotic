---
name: architect
description: >
  Project overview, structure, and conventions for
  kozmotic. Use when planning features, onboarding, or
  making architectural decisions.
invocation: >
  Use /architect to get a full project briefing before
  planning or designing a new feature.
---

# Kozmotic Architecture Guide

## Purpose

Kozmotic is a Rust CLI that provides **agent-friendly
tools** — small, portable commands designed to be called
by AI agents (Claude Code hooks, MCP servers, automation
scripts) and also used standalone by humans.

Every command outputs a consistent JSON envelope so
agents can parse results without guessing. A `--format
human` flag switches to readable text for interactive
use.

## Project Identity

| Field | Value |
|-------|-------|
| Language | Rust (edition 2024, stable toolchain) |
| Binary | `kozmotic` |
| License | MIT (CC0 for embedded sounds) |
| Version | `Cargo.toml` is single source of truth |
| Versioning | SemVer 2.0.0 |
| Platforms | Linux, Windows, macOS (CI on all three) |

## Repository Layout

```
kozmotic/
  .claude/
    skills/           # slash-command skills
      agent-cli/      # patterns for new subcommands
      architect/      # this file
      commit/         # /commit workflow
  .github/
    workflows/
      ci.yml          # test + fmt + clippy on 3 OS
  assets/
    sounds/           # embedded CC0 OGG files
  src/
    main.rs           # all code (single-file for now)
  tests/
    integration_test.rs
  Cargo.toml
  CHANGELOG.md        # Keep a Changelog format
  CLAUDE.md           # project instructions
  LICENSE
  TODO.md             # ideas and upcoming tasks
```

## Core Abstractions

### `Cli` (clap derive)

Top-level parser. Global flags live here.

| Flag | Short | Purpose |
|------|-------|---------|
| `--format` | `-f` | `json` (default) or `human` |

### `Commands` enum

Each subcommand is a variant. Current commands:

| Command | Purpose |
|---------|---------|
| `example` | Placeholder hello-world |
| `agent-ping` | Play notification sounds |

### `Output<T>` envelope

```json
{
  "status": "success" | "error",
  "data": { /* command-specific */ },
  "metadata": {
    "timestamp": "RFC 3339",
    "tool": "command-name",
    "version": "0.1.0"
  }
}
```

Construct with:
- `Output::success("tool", data)` — stdout
- `Output::error("tool", "CODE", "msg")` — stderr

### Error pattern

1. Define a `thiserror` enum per command
   (e.g., `AgentPingError`).
2. Add `.code()` returning `UPPER_SNAKE_CASE` string.
3. Add `.exit_code()`: 1 for user errors,
   2 for system errors.
4. Use `emit_error(format, &err)` to output and return
   the `ExitCode`.

### Stdout / stderr discipline

- **stdout** — structured data only (JSON or human).
- **stderr** — errors, diagnostics, warnings.
- Never mix the two.

## Subcommands in Detail

### `agent-ping`

Plays notification sounds. Three input modes
(mutually exclusive via clap `group = "source"`):

| Flag | Mode |
|------|------|
| `--sound <preset>` | Embedded OGG preset |
| `--frequency <Hz>` | Generated sine tone |
| `--file <path>` | Custom audio file |

Presets (case-insensitive, named after Claude Code
hook events):

| Preset | Sound file | Use case |
|--------|-----------|----------|
| `PostToolUse` | beep.ogg | After tool call |
| `Stop` | message-sent.ogg | Agent finished |
| `SubagentStop` | message-sent.ogg | Subagent done |
| `TaskCompleted` | message-sent.ogg | Task complete |
| `Notification` | message.ogg | Attention needed |

Additional flags: `--volume`, `--repeat`, `--interval`,
`--duration`, `--list`, `--dry-run`.

Sounds are embedded at compile time via
`include_bytes!` (~99 KB total). Audio playback uses
`rodio` 0.21 with the `playback` + codec features.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI parsing (derive mode) |
| `serde` + `serde_json` | JSON serialisation |
| `chrono` | Timestamps |
| `thiserror` | Error enums |
| `anyhow` | Ad-hoc errors (not yet used) |
| `rodio` | Audio playback |

Dev: `assert_cmd`, `predicates`.

## Development Practices

- **DDD** — design driven by domain concepts.
- **TDD** — write failing tests first, then implement.
- **Conventional Commits** — see `/commit` skill.
- **Ask before assuming** — when multiple approaches
  exist, ask interactively.
- **80-column markdown** — wrap all `.md` at 80 cols.

## Quality Gates

Before any task is complete:

1. `cargo test` — all tests pass
2. `cargo clippy --all-targets -- -D warnings` — clean
3. `cargo fmt --all -- --check` — formatted
4. Coverage >= 95%

## CI Pipeline

GitHub Actions (`.github/workflows/ci.yml`):

| Job | Runs on | What |
|-----|---------|------|
| test | ubuntu, windows, macos | `cargo test` |
| fmt | ubuntu | `cargo fmt --check` |
| clippy | ubuntu | `cargo clippy -D warnings` |

Linux jobs install `libasound2-dev` for rodio.

## Versioning and Releases

- **SemVer**: MAJOR (breaking CLI/JSON changes),
  MINOR (new commands/flags), PATCH (bug fixes).
- **`Cargo.toml`** is the single source of truth.
- **`CHANGELOG.md`** uses Keep a Changelog format.
  Always has an `[Unreleased]` section at the top.
- On release: rename `[Unreleased]` to
  `[X.Y.Z] - YYYY-MM-DD`, add fresh `[Unreleased]`.

## Planning

`TODO.md` tracks ideas and upcoming tasks. Check it
before starting new work and update it as items are
completed or added.

## Adding a New Subcommand

See the `/agent-cli` skill for detailed patterns.
Summary:

1. Add a `Commands` variant with clap attributes.
2. If many args, create an `Args` struct to avoid
   clippy's `too_many_arguments` lint.
3. Handle in `main()` match — both JSON and human
   output paths.
4. Define a `thiserror` error enum with `.code()`
   and `.exit_code()`.
5. Write integration tests: JSON output, human output,
   error cases, edge cases.
6. Update `CHANGELOG.md` under `[Unreleased]`.
7. Run quality gates before finishing.

## Design Principles

- **Agent-first**: JSON output is the primary format;
  human output is a courtesy. Agents should never need
  to scrape human text.
- **No interactive prompts**: all input via flags/args.
  For destructive ops, require `--yes` or treat as
  dry-run.
- **Portable**: must build and pass CI on all three
  platforms.
- **Minimal**: no feature flags, no optional deps,
  no plugin system. Add complexity only when a concrete
  command demands it.
- **Single binary**: everything compiles into one
  `kozmotic` binary. No runtime config files or
  external assets.
