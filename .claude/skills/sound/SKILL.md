---
name: sound
description: Toggle hook sounds on or off.
invocation: >
  /sound off to mute, /sound on to unmute.
  No restart needed.
---

# Sound Toggle

Mute or unmute `agent-ping` hook sounds by
creating or removing `~/.claude/.mute-sounds`.

## Usage

- `/sound off` — mute all hook sounds
- `/sound on` — unmute hook sounds

## Steps

1. Parse the argument (`on` or `off`). If missing,
   check the current state and report it.

2. **off** — create the mute file:
   ```bash
   touch ~/.claude/.mute-sounds
   ```

3. **on** — remove the mute file:
   ```bash
   rm -f ~/.claude/.mute-sounds
   ```

4. Confirm the new state to the user.

## How it works

`agent-ping` checks for `~/.claude/.mute-sounds`
at runtime. If the file exists, it exits silently
with a `"muted": true` response instead of playing
audio. No restart of the Claude Code session is
needed — the change takes effect on the next hook
invocation.
