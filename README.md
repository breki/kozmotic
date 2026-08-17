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

This copies the binary to `~/.claude/bin/kozmotic` so
Claude Code hooks and the status line can reference it by
a stable path:

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

#### Prebuilt platforms

| Archive target | Runs on |
|----------------|---------|
| `x86_64-unknown-linux-gnu` | 64-bit Intel/AMD Linux, glibc 2.35+ |
| `aarch64-unknown-linux-gnu` | 64-bit ARM Linux, glibc 2.35+ |
| `x86_64-pc-windows-msvc` | Windows 10/11, 64-bit |
| `aarch64-apple-darwin` | macOS on Apple silicon |
| `x86_64-apple-darwin` | macOS on Intel |

Every archive ships the binary, the licence, and an
`INSTALL.md` written for that platform alone. The same
guides live here:
[Linux](docs/install/linux.md) ·
[Windows](docs/install/windows.md) ·
[macOS](docs/install/macos.md).

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
| `kozmotic sessions prompts` | List a session's user prompts from the transcript store |
| `kozmotic self install` | Install the binary into `~/.claude/bin/` |

### `status-line`

Reads Claude Code session JSON on stdin and renders a
status bar line.

```bash
kozmotic status-line \
  --show model,context,cost,git-branch,git-lines \
  --separator " | "
```

| Flag | Purpose | Default |
|------|---------|---------|
| `--show` | Widget layout (see below) | `model,context,cost` |
| `--separator` | Text between widgets | `" \| "` |
| `--width` | Columns to right-align against | `COLUMNS`, else the terminal width, else 80 |

#### Layout

The `--show` value is a small layout language:

- `,` separates widgets within a group.
- `;` starts a new line of the status bar.
- `~` splits a line: widgets after it are pushed to the
  right edge.

```bash
kozmotic status-line --show 'git-branch,git-files~cost,rate-limit'
```

```
main | git 5mod 1new                cost $1.23 | 5h 31% (→19:00)
```

Padding is measured in display columns, so ANSI colours
and double-width characters do not skew the alignment.
When a line is too wide to fit, the groups are separated
by a single space instead of being truncated — an
overlong line wraps, whereas a truncated one could leave
the terminal stuck in a colour.

Right-alignment needs to know the terminal width. Claude
Code pipes the command's output, so `--width` is resolved
from the flag first, then the `COLUMNS` environment
variable, then the controlling terminal, then 80. Pass
`--width` explicitly if the result looks off.

Wire it up in `settings.json` (or run
`/statusline-setup`):

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/bin/kozmotic status-line --show 'host,ram,disk;model,context~cost,rate-limit'"
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
| `env:VAR` | Value of an environment variable | `bombyx-host` |
| `env:VAR:label` | Same, behind a dimmed label | `vm bombyx-host` |

`env:VAR` shows a value kozmotic knows nothing about — the
VM a session runs on, a deployment target, a cluster name.
The variable has to be visible to the process Claude Code
spawns, so export it in the shell you launch Claude Code
from; setting it afterwards or in another terminal does
nothing. When the widget renders nothing, check
`echo $VAR` in that same shell first. An unset or blank
variable renders nothing at all; `env:` with no variable
name is rejected like any other misspelt widget. Control
characters in the value are stripped and the value is
capped at 120 characters, so a stray escape sequence
cannot recolour the bar. A label may contain `:`, but not
`,`, `;` or `~` — the `--show` grammar claims those
first.

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

### `agent-ping`

Plays a notification sound. Built for Claude Code hooks,
where an audible cue tells you a long run has finished or
needs input.

```bash
kozmotic agent-ping --sound Stop        # built-in preset
kozmotic agent-ping --file chime.wav    # your own audio
kozmotic agent-ping --frequency 440     # generated tone
kozmotic agent-ping --list              # list presets
```

Presets are named after the hook events they serve:
`Stop`, `StopFailure`, and `Notification`. Preset names
are matched case-insensitively.

| Flag | Purpose | Default |
|------|---------|---------|
| `--volume` | Playback volume, 0.0–1.0 | `0.5` |
| `--repeat` | Play N times | `1` |
| `--interval` | Gap between repeats, ms | `100` |
| `--duration` | Tone length, ms (`--frequency` only) | `200` |
| `--dry-run` | Report what would play, make no sound | off |

Supported file formats: WAV, MP3, Ogg Vorbis, and FLAC.

**Muting.** If `~/.claude/.mute-sounds` exists, playback
is skipped silently and the command still succeeds — so a
muted machine never breaks a hook. The `/sound` skill
toggles that file.

### `sessions prompts`

Lists the prompts you sent in a Claude Code session,
read from the transcripts Claude Code keeps under
`~/.claude/projects/` (or `$CLAUDE_CONFIG_DIR`).

```bash
kozmotic sessions prompts                    # this session
kozmotic sessions prompts --limit 10         # the last 10
kozmotic sessions prompts --no-commands      # typed text only
kozmotic sessions prompts --session <id>     # a specific one
kozmotic sessions prompts --project ~/other  # another project
```

With no arguments it reads the session it is running
inside, taken from the `CLAUDE_CODE_SESSION_ID` that
Claude Code exports to every command it spawns — so an
agent can inspect its own transcript without being told
where it lives. Outside a session it falls back to the
current project's most recently modified transcript.

A transcript's `user` records cover far more than user
input: tool results, replayed system notices, and the
local-command plumbing behind slash commands all carry
the same label. Only what you actually typed is kept.
Slash commands are kept too, tagged `kind: "command"`
with the command in its own field:

```json
{
  "index": 14,
  "kind": "command",
  "text": "",
  "command": "/release",
  "timestamp": "2026-08-14T20:32:11.004Z",
  "git_branch": "main"
}
```

`index` numbers a prompt within its session and counts
commands whether or not they are shown, so it identifies
the same prompt under any combination of filters.
`--limit` trims the listing without renumbering it.

Human output is one row per prompt, with multi-line
prompts shown by their first line:

```
  13  2026-08-14T20:31  Go
  14  2026-08-14T20:32  /release
  16  2026-08-16T05:30  What are those two todos?
```

### `self install`

```bash
kozmotic self install                    # ~/.claude/bin/
kozmotic self install --target-dir /opt  # somewhere else
```

Copies the running binary to the target directory and
makes it executable, giving hooks and the status line a
stable path that survives rebuilds.

## Output Format

All tools output JSON by default:

```json
{
  "status": "success",
  "data": { ... },
  "metadata": {
    "timestamp": "2026-02-15T20:00:00Z",
    "tool": "example",
    "version": "1.2.1"
  }
}
```

Pass `--format human` for readable output instead.
`status-line` is the exception: it renders a status bar
line rather than an envelope, since Claude Code consumes
its stdout directly.

## Development

Build tasks go through the `xtask` crate rather than raw
cargo commands, so the same checks run locally and in CI:

```bash
cargo xtask check       # fast compile check
cargo xtask test        # tests only
cargo xtask clippy      # lints only
cargo xtask fmt         # format the code
cargo xtask validate    # fmt + clippy + tests + coverage
cargo xtask coverage    # coverage only (>= 90%)
cargo xtask dupes       # duplication check (<= 6%)
cargo xtask changelog   # add a CHANGELOG entry
cargo xtask todo        # list/add/done TODO items
```

`cargo xtask validate` is the acceptance gate: formatting,
`-D warnings` clippy, the full test suite, and a 90% line
coverage floor (85% per module). On Windows, `.\build.ps1
validate` wraps the same pipeline.

Hardware- and network-bound leaf modules are exempt from the
coverage gate via `[workspace.metadata.coverage]` in the root
`Cargo.toml`; see CLAUDE.md for the recipe.

Commits go through `/commit`, which spawns two read-only
reviewer agents in parallel over the staged diff -- `red-team`
(security and correctness) and `artisan` (code quality) --
before anything is committed.

## Contributing

Contributions are welcome! Please feel free to submit a
Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Roadmap

Shipped:

- [x] Core CLI framework with structured JSON output
- [x] Sound notifications for hooks (`agent-ping`)
- [x] Claude Code status line (`status-line`)
- [x] Session transcript queries (`sessions prompts`)
- [x] Self-installation (`self install`)

Ideas, not commitments:

- [ ] File system operations tool
- [ ] Process management tool
- [ ] Network utilities
- [ ] Git operations tool
- [ ] Data transformation tools
- [ ] CI/CD integrations

`TODO.md` tracks what is actually queued next.

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
