# Installing kozmotic on macOS

Kozmotic is a portable CLI toolkit for AI agents. This
archive contains a prebuilt binary — no Rust toolchain or
compiler is required.

Pick the archive matching your Mac: `aarch64` for Apple
silicon (M1 and later), `x86_64` for Intel. `uname -m`
prints `arm64` or `x86_64` respectively.

## 1. Extract, clear the quarantine flag, install

The binaries are **not signed or notarized**, so Gatekeeper
quarantines them on download. Clear the flag before the
first run, or macOS will refuse to launch the binary.

```bash
tar xzf kozmotic-*-apple-darwin.tar.gz
cd kozmotic-*-apple-darwin
xattr -d com.apple.quarantine kozmotic
./kozmotic self install
```

`xattr` reports "No such xattr" if the flag was never set
(for example when the archive came from `curl`); that is
harmless.

`self install` copies the binary to `~/.claude/bin/kozmotic`
and makes it executable, so Claude Code can reference it by
a stable path.

Verify:

```bash
~/.claude/bin/kozmotic --version
```

If you skipped the `xattr` step and macOS already blocked
the binary, allow it under **System Settings → Privacy &
Security**, where a "kozmotic was blocked" notice appears
with an **Open Anyway** button.

## 2. Wire it into Claude Code

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

The first sound may prompt for microphone-adjacent audio
permissions on some macOS versions; kozmotic only plays
audio, it never records.

## 3. Everyday use

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

**"kozmotic" cannot be opened because the developer cannot
be verified.**
The quarantine flag is still set — see step 1.

**`bad CPU type in executable`.**
You downloaded the archive for the other architecture.
Check with `uname -m` and download the matching one.

**The status line shows nothing.**
Run the command by hand to see the error it would print:

```bash
echo '{}' | ~/.claude/bin/kozmotic status-line --show model
```

## More

Source, full widget reference, and issue tracker:
<https://github.com/breki/kozmotic>
