---
name: statusline-setup
description: >
  Configure the Claude Code status line to use
  kozmotic status-line. Walks the user through
  choosing widgets and updates settings.json.
invocation: >
  /statusline-setup to start the interactive
  walkthrough. Optionally pass widgets directly:
  /statusline-setup model,context,cost
---

# Status Line Setup

Interactive walkthrough to configure `kozmotic
status-line` for the Claude Code status bar.

## Steps

1. **If no argument was passed**, run the
   interactive walkthrough (steps 2-4). If a
   comma-separated widget list was passed as an
   argument, skip to step 5.

2. **Present available widgets** using
   AskUserQuestion with `multiSelect: true`:

   Ask: "Which widgets do you want in your status
   line? Select all that apply."

   Options (use these exact labels and descriptions):
   - `model` — "Current model name
     (e.g. Opus 4.6)"
   - `context` — "Context window usage %,
     color-coded green/yellow/red"
   - `cost` — "Session cost in USD
     (e.g. $1.23)"
   - `lines` — "Lines of code added/removed
     (e.g. +150/-30)"
   - `rate-limit` — "5-hour API rate limit
     usage % (Pro/Max only)"
   - `vim` — "Vim mode indicator
     (NORMAL/INSERT)"

3. **Ask about separator** using AskUserQuestion:

   Ask: "What separator between widgets?"

   Options:
   - ` | ` (pipe, the default)
   - ` · ` (middle dot)
   - `  ` (two spaces)
   - Other (let user type custom)

4. **Ask about scope** using AskUserQuestion:

   Ask: "Where should this be configured?"

   Options:
   - "Global (Recommended)" —
     `~/.claude/settings.json`, applies to all
     projects
   - "This project only" —
     `.claude/settings.json`, only this repo

5. **Read the target settings.json** file.

6. **Check for existing statusLine config.** If
   one exists, show the current command and ask
   whether to replace it.

7. **Build the command string:**
   ```
   ~/.claude/bin/kozmotic status-line --show <widgets> --separator "<sep>"
   ```
   Omit `--separator` if the user chose the
   default (` | `).

8. **Set the `statusLine` field** in settings.json:
   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "<command string from step 7>"
     }
   }
   ```
   Preserve all other existing settings.

9. **Write the updated file.**

10. **Show a summary** to the user:
    - Which widgets were selected
    - Which settings file was updated
    - Remind them to restart Claude Code (or
      start a new session) for changes to take
      effect
    - Mention `/sound off` if they want to
      temporarily mute hook sounds

## Rules

- Never overwrite existing settings — merge only
  the `statusLine` key.
- If `statusLine` is already configured, show the
  current value and ask before replacing.
- Use AskUserQuestion for all interactive steps.
