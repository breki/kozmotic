# Installing kozmotic on Windows

Kozmotic is a portable CLI toolkit for AI agents. This
archive contains a prebuilt binary — no Rust toolchain or
compiler is required.

Requirements: 64-bit Windows 10 or 11. Nothing else needs
installing.

## 1. Extract and install

Unblock the downloaded zip first — Windows marks files
from the internet, and the mark is inherited by every file
extracted from them.

```powershell
Unblock-File .\kozmotic-*-windows-msvc.zip
Expand-Archive .\kozmotic-*-windows-msvc.zip -DestinationPath .
cd .\kozmotic-*-windows-msvc
.\kozmotic.exe self install
```

`self install` copies the binary to
`%USERPROFILE%\.claude\bin\kozmotic.exe`, so Claude Code
can reference it by a stable path.

Verify:

```powershell
& "$env:USERPROFILE\.claude\bin\kozmotic.exe" --version
```

If SmartScreen blocks the first run, choose **More info →
Run anyway**. The binaries are not code-signed.

## 2. Wire it into Claude Code

Both of these go in `%USERPROFILE%\.claude\settings.json`.
Claude Code expands `~` on Windows too, so the paths below
work as written.

**Status line** — renders session info in the status bar:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/bin/kozmotic.exe status-line --show 'host,ram,disk;model,context,cost,git-branch'"
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
        "command": "~/.claude/bin/kozmotic.exe agent-ping --sound Stop"
      }
    ]
  }
}
```

Note the `.exe` suffix in both commands.

## 3. Everyday use

```powershell
kozmotic --help                  # all subcommands
kozmotic agent-ping --list       # available sound presets
kozmotic status-line --help      # widget and format options
```

Every tool prints JSON by default so it composes inside
hooks and scripts; pass `--format human` for readable
output.

To call `kozmotic` from any directory, add
`%USERPROFILE%\.claude\bin` to your `PATH`:

```powershell
[Environment]::SetEnvironmentVariable(
  "Path",
  "$env:Path;$env:USERPROFILE\.claude\bin",
  "User")
```

Open a new terminal for the change to take effect.

## Uninstalling

```powershell
Remove-Item "$env:USERPROFILE\.claude\bin\kozmotic.exe"
```

Then remove the `statusLine` and `hooks` entries you added
to `settings.json`.

## Troubleshooting

**The status line shows nothing.**
Run the command by hand to see the error it would print:

```powershell
'{}' | & "$env:USERPROFILE\.claude\bin\kozmotic.exe" `
  status-line --show model
```

**"cannot be loaded because running scripts is disabled".**
That is PowerShell's execution policy blocking a script,
not kozmotic — the commands above are single invocations
and are unaffected.

## More

Source, full widget reference, and issue tracker:
<https://github.com/breki/kozmotic>
