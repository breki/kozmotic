---
name: commit
description: >
  Stage and commit changes following kozmotic project
  conventions (Conventional Commits, structured footer).
invocation: >
  Use /commit to commit staged/unstaged changes. Optionally
  pass a hint, e.g. /commit fix login bug.
---

# Commit Skill

Create a git commit that follows project conventions.

## Steps

1. **Inspect the working tree**

   Run in parallel:
   - `git status` — list changed and untracked files.
   - `git diff` — see unstaged changes.
   - `git diff --cached` — see already-staged changes.
   - `git log --oneline -5` — recent commits for style
     reference.

2. **Decide what to stage**

   - Stage only files relevant to one logical change.
   - Prefer `git add <file>...` over `git add -A`.
   - Never stage secrets (`.env`, credentials, keys).
   - If the working tree contains multiple unrelated
     changes, ask the user which to include.

3. **Run pre-commit checks**

   Before committing, verify the code is clean:
   ```
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
   If any check fails, fix the issue first (or ask the
   user) — do not skip hooks or checks.

4. **Draft the commit message**

   Follow Conventional Commits format:

   ```
   type(scope): subject

   Body text here.

   AI-Generated: Claude Code (<ModelName> <Date>)
   ```

   ### Header — `type(scope): subject`

   - **50 characters max** (including type and scope).
   - Imperative mood ("add" not "added").
   - No period at the end.
   - Common types: `feat`, `fix`, `refactor`, `test`,
     `docs`, `chore`, `ci`, `style`, `perf`, `build`.
   - Scope is optional; use the module or area touched
     (e.g., `cli`, `output`, `ci`).

   ### Body

   - Wrap at 72 characters.
   - Explain *what* changed and *why*, not *how*.
   - Separate from header with a blank line.
   - May be omitted for trivial changes.

   ### Footer

   - Always include:
     `AI-Generated: Claude Code (<ModelName> <Date>)`
     where `<ModelName>` is the current model (e.g.,
     `Opus 4.6`) and `<Date>` is today's date
     (`YYYY-MM-DD`).
   - Add `Refs: PROJ-123` only if a Jira ticket exists;
     omit otherwise.

   ### Prohibited lines

   - Do **NOT** add `Co-Authored-By` lines.
   - Do **NOT** add `Generated with Claude Code` lines.

5. **Create the commit**

   Use a heredoc so the message formats correctly:

   ```bash
   git commit -m "$(cat <<'EOF'
   type(scope): subject

   Body text.

   AI-Generated: Claude Code (<ModelName> <Date>)
   EOF
   )"
   ```

   Always create a **new** commit. Never amend a previous
   commit unless the user explicitly asks.

6. **Verify**

   Run `git status` after committing to confirm a clean
   state. Report the commit hash and message to the user.

## Rules

- Never `git push` unless the user explicitly asks.
- Never use `--no-verify` or `--no-gpg-sign`.
- Never force-push.
- If a pre-commit hook fails, fix and create a **new**
  commit — do not `--amend` (the failed commit never
  happened).
- If unsure about what to include or how to phrase the
  message, ask the user.
