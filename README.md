# Kozmotic

**AI agent-friendly, fast and portable CLI tools written
in Rust**

Kozmotic provides a collection of command-line tools
designed to be easily consumed by AI agents, automation
scripts, and other programmatic interfaces. All tools
output structured data (JSON) by default, with
human-readable options available.

The project dogfoods its own tools — kozmotic CLI tools
are used within the project's own development workflow.

## Features

- **Structured Output**: JSON output by default for easy
  parsing
- **Agent-Friendly**: Designed for consumption by AI agents
  and automation
- **Modular Tools**: Each tool focuses on a specific task
- **Fast & Reliable**: Built in Rust for performance and
  safety
- **Consistent Interface**: Uniform command structure across
  all tools

## Installation

### Quick install (from GitHub Releases)

No Rust toolchain is needed on the target machine —
releases ship prebuilt binaries. Download the archive for
your platform from
[GitHub Releases](https://github.com/breki/kozmotic/releases),
extract it, then install it to `~/.claude/bin/`:

```bash
./kozmotic self install
```

#### Prebuilt platforms

| Archive target | Runs on |
|----------------|---------|
| `x86_64-unknown-linux-gnu` | 64-bit Intel/AMD Linux, glibc 2.35+ |
| `aarch64-unknown-linux-gnu` | 64-bit ARM Linux, glibc 2.35+ |
| `x86_64-pc-windows-msvc` | Windows 10/11, 64-bit |
| `aarch64-apple-darwin` | macOS on Apple silicon |
| `x86_64-apple-darwin` | macOS on Intel |

#### Runtime requirements

- **Linux: ALSA must be installed.** The binary links
  `libasound.so.2` for `agent-ping`, and a missing
  library stops it from starting *at all* — including
  `status-line`. Install `libasound2` (Debian/Ubuntu) or
  `alsa-lib` (Fedora/Arch). Desktop installs already
  have it; minimal server images often do not.
- **Linux: glibc 2.35 or newer** (Ubuntu 22.04+,
  Debian 12+, RHEL 9+). Older distributions need a build
  from source.
- **macOS: the binaries are not signed or notarized.**
  Gatekeeper quarantines them on first run; clear it
  with `xattr -d com.apple.quarantine kozmotic`.

This copies the binary to `~/.claude/bin/kozmotic` so
Claude Code hooks can reference it directly:

```json
{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "~/.claude/bin/kozmotic agent-ping --sound Stop"
      }
    ]
  }
}
```

### From source (development)

```bash
git clone https://github.com/breki/kozmotic.git
cd kozmotic
cargo install --path crates/kozmotic
```

## Usage

```bash
# Get help
kozmotic --help

# Run a tool
kozmotic <tool-name> [OPTIONS]
```

## Tools

| Command | Purpose |
|---------|---------|
| `kozmotic example` | Reference subcommand for new tools |
| `kozmotic agent-ping` | Play notification sounds (presets, files, tones) |
| `kozmotic status-line` | Format Claude Code session JSON for the status bar |
| `kozmotic self install` | Install the binary into `~/.claude/bin/` |

### `status-line`

Reads Claude Code session JSON on stdin and renders a
status bar line.

```bash
kozmotic status-line \
  --show model,context,cost,git-branch,git-lines \
  --separator " | "
```

`--show` defaults to `model,context,cost`; `--separator`
defaults to `" | "`. Use a newline in the separator for a
multi-line status bar.

Wire it up in `settings.json` (or run
`/statusline-setup`):

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/bin/kozmotic status-line --show model,context,cost,git-branch"
  }
}
```

#### Widgets

Widgets that would render empty are omitted
automatically, so a clean working tree or a missing
session field costs nothing.

| Widget | Shows | Example |
|--------|-------|---------|
| `model` | Model display name | `Opus 5` |
| `context` | Context used %, green/yellow/red at 50/80 | `ctx 42.5%` |
| `cost` | Session cost in USD | `cost $1.23` |
| `cost-rate` | Burn rate per wall-clock hour | `rate $4.20/h` |
| `lines` | Session lines added/removed | `+150/-30` |
| `duration` | Wall-clock session time | `time 12m 5s`, `2h 15m`, `1d 3h` |
| `api-duration` | Time spent in API calls | `api 3m 20s` |
| `tokens` | Input/output tokens, k/M scaled | `tok 1.2M in / 45.0k out` |
| `git-branch` | Current branch | `main` |
| `git-ahead` | Commits ahead/behind upstream | `↑2 ↓1` |
| `git-files` | Staged/modified/new/deleted counts | `git 2staged 1mod`, `git (clean)` |
| `git-lines` | Uncommitted added/deleted lines | `+42/-7` |
| `last-commit` | Relative age of `HEAD` | `last 12m`, `last 2h 15m`, `last 3d 4h` |
| `git-status` | Compact staged/modified | `+2 ~1` |
| `directory` | Basename of the current directory | `kozmotic` |
| `session` | First 8 chars of the session id | `sid 33f12afa` |
| `rate-limit` | 5-hour quota % and reset time | `5h 53% (→21:00)` |
| `rate-limit-7d` | 7-day quota % and reset time | `7d 71% (→Thu 21:00)` |
| `vim` | Vim mode indicator | `NORMAL` |
| `worktree` | Active worktree name | `wt feature-x` |
| `agent` | Active agent name | `agent Explore` |
| `api-status` | status.claude.com health, cached 2 min | `api ok`, `api degraded`, `api outage` |
| `host` | Machine's short host name | `host devbox` |
| `ram` | RAM used/installed, colored at 50/80% | `ram 12.4/31.3G` |
| `disk` | Disk used/total for the session's filesystem | `disk 210/468G` |

All `git-*` widgets share a per-render cache, so each
underlying `git` command runs at most once regardless of
how many are configured. The same applies to `host`,
`ram`, and `disk`, which probe the system at most once
per render.

`disk` reports the filesystem holding the session's
working directory (`workspace.current_dir`, falling back
to the process's own directory) — on a machine with a
separate `/home` or a mounted project volume, that is the
volume you are actually filling up. Sizes are binary
(1 G = 1024 M).

`api-status` performs an HTTP request (cached for two
minutes) — omit it if you want a fully offline status
line. It never renders empty: if status.claude.com cannot
be reached it shows the last known value with a trailing
`~` (`api outage~`), or `api unknown` when nothing was
cached. Failed lookups are retried at most every 30
seconds, and each request is capped at 2.5 seconds so a
status-page outage cannot stall the status line.

## Output Format

All tools output JSON by default:

```json
{
  "status": "success",
  "data": { ... },
  "metadata": {
    "timestamp": "2026-02-15T20:00:00Z",
    "tool": "example",
    "version": "0.1.0"
  }
}
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run locally
cargo run -- --help
```

## Contributing

Contributions are welcome! Please feel free to submit a
Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Roadmap

- [ ] Core CLI framework
- [ ] File system operations tool
- [ ] Process management tool
- [ ] Network utilities
- [ ] Git operations tool
- [ ] Data transformation tools
- [ ] CI/CD integrations

## Why Kozmotic?

Traditional CLI tools are designed for human interaction,
which can make them difficult for agents to parse:
- Inconsistent output formats
- Mixed structured and unstructured data
- Varying exit codes and error reporting
- Different conventions across tools

Kozmotic solves this by providing a consistent, structured
interface across all tools, making automation and AI agent
integration seamless.
