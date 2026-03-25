---
name: todo
description: >
  Process the next pending TODO item from TODO.md.
  Asks clarifying questions, implements the item,
  then moves it to the Done section.
disable-model-invocation: true
---

## Steps

1. Read `TODO.md` and identify the first pending
   item (items under "Done" are already completed).

2. If the item is ambiguous or has multiple possible
   approaches, use AskUserQuestion to clarify before
   starting work.

3. Implement the item. Follow all project rules
   from CLAUDE.md (TDD, DDD, etc.).

4. Run the acceptance checks:
   ```
   cargo xtask validate
   ```

5. Move the completed item from the pending list
   to a `## Done` section at the bottom of
   `TODO.md`. Add the completion date in
   parentheses, e.g.:
   ```
   ## Done
   - extract modules from main.rs (2026-03-25)
   ```
   Create the `## Done` section if it doesn't exist.

6. Run `/commit` to commit the changes.
