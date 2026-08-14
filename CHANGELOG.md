# Changelog

All notable changes to this project will be documented in
this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Release builds for `aarch64-unknown-linux-gnu`, so
  ARM Linux machines (Raspberry Pi, Ampere/Graviton
  VMs) get a prebuilt binary instead of having to
  build from source.
- README documents the prebuilt platforms and their
  runtime requirements: the ALSA shared library on
  Linux, the minimum glibc, and clearing the macOS
  Gatekeeper quarantine attribute.

### Changed

- Linux release binaries are now built on
  `ubuntu-22.04` instead of `ubuntu-latest`. The
  runner's glibc becomes the minimum glibc of every
  machine that can run the artifact, so pinning drops
  the floor from 2.39 to 2.35 (Ubuntu 22.04+,
  Debian 12+, RHEL 9+).

## [1.2.0] - 2026-08-14

### Added

- `host` status-line widget shows the machine's short
  host name, e.g. `host devbox`.
- `ram` status-line widget shows RAM used against
  installed, e.g. `ram 12.4/31.3G`, colored green/
  yellow/red at 50%/80% like `context`.
- `disk` status-line widget shows used against total
  space on the filesystem holding the session's
  working directory, e.g. `disk 210/468G`.

### Fixed

- `api-status` no longer disappears when
  status.claude.com cannot be reached — the exact
  case where the widget matters most. It now shows the
  last known indicator marked stale (`api outage~`),
  or `api unknown` when nothing is cached.
- `api-status` HTTP requests are now capped (1.5s
  connect, 2.5s total) and failed lookups are retried
  at most every 30 seconds, so an unreachable status
  page can no longer stall or blank the status line.

### Changed

- The `api-status` cache file now stores JSON with the
  last successful fetch and last attempt time instead
  of a bare indicator string. An older cache file is
  ignored and rewritten on first use.

## [1.1.0] - 2026-08-03

### Added

- `git-lines` status-line widget shows added/deleted
  line counts for the current uncommitted changeset
  (staged + unstaged combined), e.g. `+42/-7`. Hidden
  when the working tree is clean. Binary files are
  skipped.
- `last-commit` status-line widget shows the relative
  age of `HEAD` with minute granularity, e.g.
  `last 12m`, `last 2h 15m`, or `last 3d 4h`.
- `cost-rate` status-line widget shows session burn
  rate in dollars per wall-clock hour, e.g.
  `rate $4.20/h`.

### Changed

- `git-*` status-line widgets now share a per-render
  cache, so each underlying `git` command runs at
  most once even when several git widgets are
  configured (previously `git-lines` and `git-status`
  each spawned the same two `git diff --numstat`
  processes independently).
- `status-line` now surfaces a red diagnostic on
  stdout (in addition to stderr) when stdin is empty
  or the session JSON is malformed, so failures are
  visible in the Claude Code status bar instead of
  silently collapsing to an empty line.
- `duration` and `api-duration` status-line widgets now
  scale to hours and days for long sessions, rendering
  `Xh Ym` past one hour and `Xd Yh` past one day instead
  of unbounded minutes (e.g. `3096m 2s` → `2d 3h`).
- `rate-limit` and `rate-limit-7d` status-line widgets
  now append the local datetime when the quota window
  resets, e.g. `5h 53% (→21:00)` and
  `7d 71% (→Thu 21:00)`. Accepts `resets_at` as either
  a Unix timestamp integer, an RFC3339 string, or
  `null`. The widgets render whenever `resets_at` is
  present, even at 0% usage.

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
