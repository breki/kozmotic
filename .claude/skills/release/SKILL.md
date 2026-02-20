---
name: release
description: >
  Prepare a versioned release: bump version, finalise
  changelog, commit, and tag.
invocation: >
  Use /release to cut a new release. Optionally pass the
  bump level, e.g. /release minor or /release 0.3.0.
---

# Release Skill

Prepare and tag a release following project conventions.

## Prerequisites

Before starting a release:

- Working tree must be **clean** (no uncommitted
  changes). If it is not, ask the user to commit or
  stash first — do not commit unrelated work as part
  of the release.
- The `[Unreleased]` section in `CHANGELOG.md` must
  have content. If it is empty, warn the user and ask
  whether to proceed with an empty release.

## Steps

1. **Inspect the current state**

   Run in parallel:
   - `git status` — verify clean working tree.
   - Read `Cargo.toml` — get current version.
   - Read `CHANGELOG.md` — check `[Unreleased]`
     section has entries.
   - `git tag --list 'v*'` — list existing tags to
     avoid conflicts.

2. **Determine the new version**

   If the user passed a version or bump level as an
   argument, use it. Otherwise, analyse the
   `[Unreleased]` changelog entries and suggest:

   - **MAJOR** — if `[Unreleased]` contains breaking
     changes to CLI interface or JSON output schema.
   - **MINOR** — if it contains `Added` entries (new
     subcommands, flags, features).
   - **PATCH** — if it contains only `Fixed`, `Changed`,
     or `Removed` entries with no new features.

   Present the suggestion and ask the user to confirm
   or override.

   Accepted argument formats:
   - Bump level: `major`, `minor`, `patch`
   - Explicit version: `1.2.3` (without `v` prefix)

3. **Update `Cargo.toml`**

   Change the `version` field to the new version.
   Run `cargo check` to regenerate `Cargo.lock`.

4. **Update `CHANGELOG.md`**

   - Rename `## [Unreleased]` to
     `## [X.Y.Z] - YYYY-MM-DD` (today's date).
   - Insert a fresh `## [Unreleased]` section above it
     with no entries.

5. **Run quality gates**

   ```
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

   If any check fails, stop and fix before continuing.

6. **Stage and commit**

   Stage the modified files:
   ```
   git add Cargo.toml Cargo.lock CHANGELOG.md
   ```

   Commit with message:
   ```
   chore(release): vX.Y.Z

   AI-Generated: Claude Code (<ModelName> <Date>)
   ```

   Use a heredoc for correct formatting.

7. **Create the tag**

   ```
   git tag -a vX.Y.Z -m "vX.Y.Z"
   ```

   Use an **annotated** tag (not lightweight) so it
   shows in `git describe` and triggers the release
   workflow.

8. **Verify**

   Run in parallel:
   - `git status` — confirm clean tree.
   - `git log --oneline -3` — confirm release commit.
   - `git tag --list 'v*'` — confirm tag exists.

   Report the commit hash, version, and tag to the
   user.

9. **Remind about push**

   Tell the user:
   ```
   Ready to publish. Push commit and tag with:
     git push && git push --tags
   ```

   Do **not** push automatically — let the user decide.

## Rules

- Never push unless the user explicitly asks.
- Never create a release from a dirty working tree.
- Never skip quality gates.
- Never reuse an existing tag — if `vX.Y.Z` already
  exists, stop and ask the user.
- Always use annotated tags (`git tag -a`).
- The tag format is `vX.Y.Z` (with `v` prefix) to
  match the `release.yml` workflow trigger.
