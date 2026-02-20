# Changelog

All notable changes to this project will be documented in
this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
- Three embedded CC0 sound effects from BigSoundBank.com

## [0.1.0] - 2026-02-15

### Added

- Initial project scaffold
- `example` subcommand with JSON and human output formats
- `Output<T>` generic response wrapper
- Global `--format` flag (`json` | `human`)
- CI pipeline for Linux, Windows, and macOS
- Integration test suite
