---
name: todo
description: >
  Process the next pending item from docs/todo.md.
  Asks clarifying questions, implements the item,
  then removes it with `cargo xtask todo done`.
disable-model-invocation: true
---

## Steps

1. Read `docs/todo.md` and identify the first item
   (each item is a `### <slug>` entry).

2. If the item is ambiguous or has multiple possible
   approaches, use AskUserQuestion to clarify before
   starting work.

3. Implement the item. Follow all project rules
   from CLAUDE.md (TDD, DDD, etc.).

4. Run the acceptance checks:
   ```
   cargo xtask validate
   ```

5. Remove the completed item:
   ```
   cargo xtask todo done <slug>
   ```
   Git history and `CHANGELOG.md` record what shipped.

6. Run `/commit` to commit the changes.
