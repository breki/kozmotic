# Installing kozmotic on Linux

Kozmotic is a portable CLI toolkit for AI agents. This
archive contains a prebuilt binary — no Rust toolchain or
compiler is required.

## 1. Check the requirements

- **glibc 2.35 or newer**: Ubuntu 22.04+, Debian 12+,
  RHEL/Rocky/Alma 9+. Check yours with `ldd --version`.
- **ALSA runtime library**: the binary links
  `libasound.so.2` for sound playback. Without it,
  *nothing* runs — not even `kozmotic status-line`.

  ```bash
  sudo apt install libasound2      # Debian, Ubuntu
  sudo dnf install alsa-lib        # Fedora, RHEL
  sudo pacman -S alsa-lib          # Arch
  ```

Pick the archive matching your CPU: `x86_64` for Intel or
AMD, `aarch64` for ARM (Raspberry Pi, Ampere, Graviton).
`uname -m` tells you which you have.

## 2. Extract and install

```bash
tar xzf kozmotic-*-linux-gnu.tar.gz
cd kozmotic-*-linux-gnu
./kozmotic self install
```

`self install` copies the binary to `~/.claude/bin/kozmotic`
and makes it executable, so Claude Code can reference it by
a stable path.

Verify:

```bash
~/.claude/bin/kozmotic --version
```

## 3. Wire it into Claude Code

Both of these go in `~/.claude/settings.json`.

**Status line** — renders session info in the status bar:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/bin/kozmotic status-line --show 'host,ram,disk;model,context,cost,git-branch'"
  }
}
```

Run `/statusline-setup` inside Claude Code to pick widgets
interactively instead of editing JSON by hand.

**Notification sounds** — play a chime when a session stops
or needs attention:

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

If you are on a headless machine with no audio device, the
sound simply fails to play; it will not break the hook.

## 4. Everyday use

```bash
kozmotic --help                  # all subcommands
kozmotic agent-ping --list       # available sound presets
kozmotic status-line --help      # widget and format options
```

Every tool prints JSON by default so it composes inside
hooks and scripts; pass `--format human` for readable
output.

## Uninstalling

```bash
rm ~/.claude/bin/kozmotic
```

Then remove the `statusLine` and `hooks` entries you added
to `~/.claude/settings.json`.

## Troubleshooting

**`error while loading shared libraries: libasound.so.2`**
The ALSA runtime library is missing — see step 1.

**`version 'GLIBC_2.35' not found`**
Your distribution is older than the build target. Build
from source instead:
<https://github.com/breki/kozmotic>

**The status line shows nothing.**
Run the command by hand to see the error it would print:

```bash
echo '{}' | ~/.claude/bin/kozmotic status-line --show model
```

## More

Source, full widget reference, and issue tracker:
<https://github.com/breki/kozmotic>
