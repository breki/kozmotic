# Changelog

All notable changes to this project will be documented in
this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-03-27

### Added

- `status-line` subcommand for Claude Code status
  bar with configurable widgets (model, context %,
  cost, lines, rate-limit, vim mode, git-branch,
  git-files, git-ahead, api-status, and more) with
  ANSI color-coded output and multi-line support
- Mute-file support for `agent-ping`: create
  `~/.claude/.mute-sounds` to silence hook sounds
  without restarting the session (`/sound` skill)
- `cargo xtask validate` command for fmt + clippy +
  test + coverage reporting in one step
- Claude Code Stop hook that runs `cargo clippy` and
  `cargo test` when Rust files are modified
- Restructured `src/main.rs` into `output`, `agent_ping`,
  and `self_install` modules
- `self install` subcommand to copy the binary to
  `~/.claude/bin/` for use in Claude Code hooks
  - `--target-dir` flag to override the install directory
- `agent-ping` subcommand for playing notification sounds
  - Built-in presets named after Claude Code hook events:
    `PostToolUse`, `Stop`, `SubagentStop`,
    `TaskCompleted`, `Notification`
  - `--frequency` flag for generated tones (20–20000 Hz)
  - `--file` flag for custom audio files
  - `--dry-run` flag for silent validation
  - `--list` flag to show available presets
  - `--volume`, `--repeat`, `--interval`, `--duration`
    options
  - Case-insensitive preset name matching
- Structured error output via `Output::error()` with
  error codes on stderr
- Embedded sound effects from Pixabay for Stop,
  StopFailure, and Notification presets
  (see `CREDITS.md`)

## [0.1.0] - 2026-02-15

### Added

- Initial project scaffold
- `example` subcommand with JSON and human output formats
- `Output<T>` generic response wrapper
- Global `--format` flag (`json` | `human`)
- CI pipeline for Linux, Windows, and macOS
- Integration test suite
