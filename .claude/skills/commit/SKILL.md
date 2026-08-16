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

3. **Code review**

   Once the change is staged, spawn the **two** dedicated
   reviewer agents **in parallel** (a single message with two
   `Agent` calls). Both are read-only by construction —
   neither has `Edit`/`Write`.

   - **Red Team** (`subagent_type: red-team`) — security and
     correctness. Has `Bash`, so it runs `git diff --cached`
     and `git log` itself.
   - **Artisan** (`subagent_type: artisan`) — code quality and
     craftsmanship beyond clippy. Has no shell, so pass it the
     captured `git diff --cached` output in its spawn prompt.

   Give each a one-line description of what the change does.
   Gating rules — when to run, how to spawn, the diff-handoff
   rule, and the six labeled-bullet reporting format — live in
   `.claude/commands/code-reviewers.md`. The review criteria
   live in the agent files under `.claude/agents/`.

   **Always run both when the diff contains code** (`.rs`,
   `.toml`, `.ps1`, `.sh`, `.github/workflows/`). Never skip
   them, even for "straightforward" changes. The only
   exception is a commit with no code at all (docs-only
   markdown).

   **Cross-confirmed findings.** Scan both reports for
   overlap before presenting. Two findings are cross-confirmed
   when they describe the same root cause — same `file:line`
   (or overlapping ranges), or the same defect in different
   vocabulary. Present those under a **Cross-confirmed**
   heading; they are a markedly stronger signal than unique
   findings.

   **Truncated reviewer output.** If a reviewer's summary
   cites finding IDs whose full bodies are missing from the
   returned text, the reply was truncated. Use `SendMessage`
   to that agent (its ID is in the tool result) and ask it to
   re-emit the missing findings verbatim, *before* presenting
   to the user — otherwise real findings are silently dropped.

   **Presenting findings.** Auto-apply is the default. Most
   findings are mechanical (tighten a match, rename a local,
   fix a stale doc); apply those and announce the set so the
   user can interrupt. Escalate via `AskUserQuestion` only
   when a finding crosses a threshold:
   1. large rework (>5 files, >100 lines, or out-of-diff
      churn),
   2. two findings conflict,
   3. a genuine design tradeoff,
   4. a public-surface or breaking change (CLI flags, JSON
      output schema),
   5. a new dependency,
   6. out of scope for this commit.

   Surface **every** finding — applied or escalated — in your
   summary. Never silently drop one.

   **Deferred findings backlog.** A *fixed* finding gets no
   log entry; its resolution lives in the commit message. Only
   a finding deliberately *deferred* (real, but not fixed now)
   is logged:
   - `docs/developer/redteam-log.md` (Red Team)
   - `docs/developer/artisan-log.md` (Artisan)

   Both are newest-first; new entries go right after the
   `---`. Use a date-slug ID — `<rt|aq>-<YYYY-MM-DD>-<slug>`,
   e.g. `rt-2026-08-16-transcript-partial-line` — so there is
   no counter to maintain and the ID is greppable from commit
   messages. Each entry is the ID heading, a `**Category:**`
   line, and a short description. A later commit acting on a
   deferred item cites its ID inline ("supersedes rt-..."),
   and stages the changed backlog file. **Threshold:** if 10+
   items sit open in either backlog, tell the user a
   full-codebase review is warranted.

4. **Run pre-commit checks**

   Before committing, verify the code is clean:
   ```
   cargo xtask validate
   ```
   If any check fails, fix the issue first (or ask the
   user) — do not skip hooks or checks.

5. **Update CHANGELOG.md**

   If the commit adds features, fixes bugs, changes
   behaviour, or removes functionality, add a bullet to
   the `[Unreleased]` section of `CHANGELOG.md` under
   the appropriate heading (`Added`, `Changed`, `Fixed`,
   or `Removed`). Stage the file together with the code
   changes.

   Skip this step for commits that do not affect the
   user-visible product (e.g., `chore`, `ci`, `style`,
   `docs` that only touch `CLAUDE.md` or similar).

6. **Bump the version (when appropriate)**

   If the user explicitly asks for a release or version
   bump:

   - Determine the correct SemVer increment:
     - **MAJOR** — breaking changes to CLI interface or
       JSON output schema.
     - **MINOR** — new subcommands, flags, or
       backwards-compatible features.
     - **PATCH** — bug fixes, documentation, internal
       refactors.
   - Update `version` in `Cargo.toml` (single source of
     truth).
   - In `CHANGELOG.md`, rename `[Unreleased]` to
     `[X.Y.Z] - YYYY-MM-DD` and add a fresh
     `[Unreleased]` section above it.
   - Stage both files with the rest of the changes.

   Do **not** bump the version unless the user asks.

7. **Draft the commit message**

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

8. **Create the commit**

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

9. **Verify**

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
