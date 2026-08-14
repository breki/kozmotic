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

2. **Offer a preset** using AskUserQuestion.

   There are 25 widgets, and AskUserQuestion allows
   at most 4 options per question, so never try to
   enumerate widgets as options. Offer bundles
   instead.

   Ask: "Which status line layout do you want?"

   Options:
   - "Recommended" —
     `model,context,cost,git-branch,git-lines`
   - "Minimal" — `model,context,cost`
   - "Git-focused" —
     `git-branch,git-ahead,git-files,last-commit,context`
   - "Custom" — pick widgets individually

   If the user picks a bundle, go to step 3.

   If the user picks "Custom", print the widget
   reference table below as plain markdown and ask
   them to reply with a comma-separated list. Do not
   wrap that list in AskUserQuestion.

   ### Widget reference

   Widgets that would render empty are omitted
   automatically, so a clean tree or a missing field
   costs nothing.

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
   | `git-branch` | Current branch, cyan | `main` |
   | `git-ahead` | Commits ahead/behind upstream | `↑2 ↓1` |
   | `git-files` | Staged/modified/new/deleted counts | `git 2staged 1mod`, `git (clean)` |
   | `git-lines` | Uncommitted added/deleted lines | `+42/-7` |
   | `last-commit` | Relative age of HEAD | `last 12m`, `last 2h 15m`, `last 3d 4h` |
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

   Notes worth passing on when relevant:
   - `cost-rate` is hidden until the session has
     measurable duration.
   - `git-lines`, `git-status`, and `git-ahead` hide
     themselves when there is nothing to report, so
     the separator does not double up.
   - `rate-limit` and `rate-limit-7d` render whenever
     a reset time is present, even at 0% usage.
   - `api-status` makes an HTTP call (2-minute cache);
     skip it if the user wants a fully offline status
     line. It always renders something — `api ok~`
     means the value is stale because the status page
     was unreachable, `api unknown` means nothing is
     cached.
   - `host`, `ram`, and `disk` read local system state
     and cost nothing extra when combined. `disk`
     reports the filesystem holding the session's
     working directory.

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
