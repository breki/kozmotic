---
name: statusline-setup
description: >
  Configure the Claude Code status line to use
  kozmotic status-line. Updates settings.json with
  the statusLine command.
invocation: >
  /statusline-setup to configure, optionally pass
  widgets: /statusline-setup model,context,cost
---

# Status Line Setup

Configure Claude Code to use `kozmotic status-line`
for the status bar.

## Steps

1. Parse the argument as a comma-separated widget
   list. If no argument, use the default:
   `model,context,cost`.

   Available widgets:
   - `model` — current model name
   - `context` — context window % (color-coded)
   - `cost` — session cost in USD
   - `lines` — lines added/removed
   - `rate-limit` — 5-hour rate limit %
   - `vim` — vim mode (NORMAL/INSERT)

2. Ask the user whether to configure globally
   (`~/.claude/settings.json`) or for this project
   only (`.claude/settings.json`).

3. Read the target `settings.json` file.

4. Set the `statusLine` field:
   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "~/.claude/bin/kozmotic status-line --show <widgets>"
     }
   }
   ```
   Preserve all other existing settings.

5. Write the updated file.

6. Tell the user the status line is configured and
   will take effect on the next session (or after
   restarting Claude Code).

## Rules

- Never overwrite existing settings — merge only
  the `statusLine` key.
- If `statusLine` is already configured, show the
  current value and ask before replacing.
