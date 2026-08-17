# Template Feedback

Issues, improvements, and observations about the
[rustbase](https://github.com/breki/rustbase) template
discovered during development of this project.

Use this log to feed improvements back to the template.
Newest entries first.

---

## 2026-08-17

Found while syncing 0.4.0 -> 0.17.0. All ten are upstream,
not local.

- **`xtask/src/todo.rs` is not rustfmt-clean under the
  template's own `rustfmt.toml`.** Line 213's
  `let Some(slug) = lines[i].starts_with("- ").then(...)`
  chain reformats immediately, so the first
  `cargo xtask validate` after a sync fails at step 1 on a
  file the project never wrote. Same class as the
  `validate.rs` trailing comma logged on 2026-08-03.
  **Suggested fix:** run `cargo fmt` in the template and add
  a CI check, since this is the second time a shipped xtask
  file has failed the gate it ships.

- **The coverage gate can be silently switched off by a
  pattern the guard does not recognise.**
  `coverage::validate_ignore_patterns` rejects match-all
  patterns with `matches!(trimmed, "." | ".*" | ".+" |
  ".*?")` -- an exact-string allowlist. Anything equivalent
  passes: `src`, `crates`, `\.rs$`, `^`, `[\s\S]*`. Verified
  by putting `ignore = ['src']` in
  `[workspace.metadata.coverage]`: `cargo xtask coverage`
  measured zero files and would have reported a pass. A gate
  that can be disabled by a one-character typo in a data
  file is worse than one that cannot be configured at all.
  **Suggested fix:** check the outcome rather than the
  spelling -- fail when the collected report contains zero
  files or zero lines, whatever the pattern was.

- **`helpers.rs` does not compile for projects that adopt a
  subset of xtask.** With `[workspace.lints.rust] warnings =
  "deny"`, taking the nine modules a CLI-only project needs
  and skipping `audit`, `backfeed`, `clean_cache`, `dep_age`,
  `deploy*`, `feedback`, `frontend*` and `sync` leaves ten
  `pub` helpers unused (`fmt_bytes`, `dir_size`,
  `DirSizeWarning`, `is_reparse_or_symlink_meta`,
  `today_iso`, `civil_from_days`, `is_fence`, `FEEDBACK_REL`,
  `BACKFEED_LEDGER_REL`, `temp_scratch`) and the build fails
  outright. The workaround is ten `#[allow(dead_code)]`
  attributes, which is local drift that the next sync has to
  work around. **Suggested fix:** put `#![allow(dead_code)]` at
  the top of `helpers.rs` upstream, or split it so each
  helper sits with its consumer.

- **`validate.rs` cannot be adopted piecemeal.** It hard-codes
  `use crate::{audit, dep_age, frontend_check, frontend_dupes,
  frontend_fmt, frontend_test}` and numbers its steps 1..11
  around them, so a project without a frontend or the
  supply-chain gates cannot take any of the ergonomic
  improvements without also taking modules it has no use for.
  We kept our own `validate.rs` and hand-patched two call
  sites instead. **Suggested fix:** build the step list from
  the modules that are present rather than a fixed sequence.

- **The reviewer backlogs ship upstream's own findings.**
  `docs/developer/{redteam,artisan}-log.md` arrive populated
  with rustbase's deferred items (`rt-2026-07-16-deploy-guard-toctou`
  and similar), which are meaningless in a derived project
  and push it past the "10+ open items warrants a full
  review" threshold. We took the headers and dropped the
  entries. **Suggested fix:** ship them empty, or list them
  as create-if-absent in `/template-sync`.

- **CLAUDE.md has no conciseness instruction, and Opus 5
  needs one.** Anthropic documents that Opus 5's "default
  user-facing responses run longer than prior Opus models'"
  and that lowering `effort` "can reduce thinking volume
  without reliably shortening the visible response" -- so
  length has to be prompted for, and no amount of effort
  tuning substitutes. The template's Collaboration section
  covers *style* ("write plainly") but never *length*, so
  every derived project inherits the long default. Files
  written to disk are affected too; this project's last
  changelog update ran to 47 bullets. **Suggested fix:** add
  the two instructions from Anthropic's Opus 5 prompting
  guide -- one for response length, one for written
  deliverables -- plus a short `<tone_preference>` repeat at
  the end of the file, which that guide recommends for long
  prompts where a rule near the top fades. Lift the wording
  from this project's `CLAUDE.md`, where it is already in
  place.

- **"Narrate the work as it happens" now amplifies a model
  default instead of correcting one.** The instruction tells
  Claude to announce every meaningful step and warns against
  batching silently. That was useful when models under-
  narrated. Anthropic's Opus 5 guide reports the opposite
  behaviour -- the model "narrates readily," "tends to
  announce what it is about to do," and its per-message
  output in agentic sessions is longer than before -- and
  gives a damping instruction instead. Keeping the old
  instruction makes the model narrate even more. The same
  guide makes the general point when it says to delete
  verification instructions carried over from earlier
  models, because they compound rather than help.
  **Suggested fix:** replace the bullet with the guide's
  shape -- one sentence before the first tool call, a brief
  update only on a discovery or a change of direction, and
  lead with the outcome at the end. This project's
  `CLAUDE.md` now carries that replacement; lift it from
  there. More broadly, the
  template's model-facing instructions are worth re-reading
  against each new model's behaviour notes; a rule written to fix an old model's
  weakness can make a new model's excess worse.

- **"Write plainly" does not stop the most common
  readability fault.** The template's style rule covers word
  choice, sentence length and naming the subject, and Claude
  follows all three while still producing sentences that
  have to be decoded. The fault it misses is the metaphor
  used in place of a literal statement: "the rule pushes in
  the same direction the model already leans" instead of
  "the rule says to talk more, and the model already talks
  too much". Every word is short, there is one idea, the
  subject is named -- and the reader still has to translate
  an image before learning the fact. Abstract nouns standing
  in for actions do the same thing ("updates only on a real
  finding"). Both read as competent writing, which is why
  the existing rule does not catch them, and both cost the
  reader a step. This was found by a user pointing at three
  such sentences in a row. **Suggested fix:** add a rule
  requiring literal statements, with before/after pairs --
  abstract advice does not work here, because the model
  producing the fault already believes it is writing
  plainly. This project's `CLAUDE.md` has the rule and the
  examples; lift them from there.

- **Every dated convention says "today's date" and nothing
  says to re-check it.** A long session resolves the date
  once from its opening context and reuses it, so a session
  that crosses midnight silently stamps yesterday onto
  everything after it. Five places are affected: the
  `AI-Generated: ... (<ModelName> <Date>)` commit footer, the
  `### YYYY-MM-DD` diary headings, the reviewer backlog IDs
  (`<rt|aq>-<YYYY-MM-DD>-<slug>`), the `[X.Y.Z] - YYYY-MM-DD`
  release heading, and the trailing dates on `TODO.md` Done
  entries. Nothing looks wrong: a wrong date reads exactly
  like a right one, and the fields affected are the ones
  used to reconstruct what happened and when. It also
  mis-sorts, writing a new dated section *under* one that
  already exists for the same day.
  This session crossed midnight and only got it right
  because the harness volunteered the new date.
  **Suggested fix:** have the skills read the clock instead
  of the context -- run `date +%F` as its own step and use
  that value, rather than instructing "today's date".

- **`.gitignore` gained a redundant `**/target/`.** `target/`
  is already unanchored and matches at any depth, confirmed
  with `git check-ignore`. The pair also ignores any
  directory named `target` anywhere, so a fixture path like
  `tests/fixtures/target/` silently never stages.
  **Suggested fix:** keep the single `target/`.

---

## 2026-08-16

- **`.claude/hooks/stop-check.sh` never runs: its
  re-entry guard matches the key, not the value.** The
  guard is
  `if echo "$input" | grep -q '"stop_hook_active"'`,
  but Claude Code's Stop payload *always* contains that
  key — `false` on the first invocation and `true` only
  on re-entry. The grep therefore succeeds every time
  and the hook exits 0 before running a single check.
  Verified against the shipped script — with modified
  `.rs` files and a broken tree, this exits 0:

  ```bash
  echo '{"hook_event_name":"Stop","stop_hook_active":false}' \
    | .claude/hooks/stop-check.sh
  ```

  This has presumably been silent in every
  derived project since the guard was written — the
  hook appears configured and does nothing, so
  developers believe fmt/clippy/tests are gated
  interactively when they are not. It also makes
  commit e266c78's investment in the staged `run_stage`
  rework unreachable, and the rationale comment about
  fmt drift slipping into CI describes a guard that
  never engages. **Suggested fix:** match the value —
  `grep -qE '"stop_hook_active"[[:space:]]*:[[:space:]]*true'`
  — or parse it with `jq -e '.stop_hook_active == true'`.
  Worth adding a regression check that the hook exits 2
  for a `false` payload against a dirty tree, since the
  failure mode is silence and nothing else would catch
  a recurrence.

- **`cargo xtask changelog add` rejects entry text that
  begins with `--`.** clap parses the leading dashes as
  a flag, so
  `cargo xtask changelog add --kind changed "--format is now a ValueEnum"`
  fails with a usage error. `--` before the text works
  around it, but the argument is a free-text entry and
  users writing about CLI flags will hit this
  constantly. **Suggested fix:** mark the positional
  with `allow_hyphen_values = true`.

---

## 2026-08-03

- **`xtask/src/validate.rs` ships a trailing comma that
  fails clippy on Rust >= 1.97.** Line 73's
  `format!("{:.1}% >= {}%", r.line_pct,
  coverage::THRESHOLD,)` trips the
  `unnecessary_trailing_comma` lint added in clippy
  1.97, so `cargo clippy --all-targets -- -D warnings`
  fails in CI for any project using the template's
  xtask. **Suggested fix:** drop the trailing comma
  upstream.

- **`rust-toolchain.toml` pins `channel = "stable"`,
  which silently drifts from CI.** CI installs `stable`
  fresh on every run, but a developer's local `stable`
  is whatever `rustup update` last fetched. kozmotic
  ran a March toolchain (1.94.1) against a July CI
  stable (1.97.1), so `cargo xtask validate` passed
  locally while CI's clippy job failed on a newer
  lint -- and stayed red for three months unnoticed,
  because validate is the only gate developers see.
  **Suggested fix:** have `xtask validate` warn when
  the active toolchain is more than N weeks behind the
  latest stable release, or document that a
  `rustup update` belongs in the release checklist.

---

## 2026-05-04

- **Coverage `IGNORE_REGEX` is a hardcoded const in
  `xtask/src/coverage.rs`.** Every project that needs
  to extend it (e.g. to exclude hardware- or network-
  bound modules) has to fork the xtask source. Real
  projects almost always need this -- kozmotic added
  `agent_ping/playback.rs` and
  `status_line/api_status.rs` to the regex during
  initial migration. **Suggested fix:** read additional
  patterns from a workspace-level config such as
  `[workspace.metadata.coverage] ignore = [...]`, or
  expose a `cargo xtask coverage --extra-ignore` CLI
  flag. The hardcoded const can stay as the defaults.

- **No documented pattern for hardware-bound code in
  the 90% gate.** The template's coverage gate assumes
  everything is testable, but many CLI projects have
  I/O paths (audio, network, native APIs) that can't
  run in CI. kozmotic settled on: extract into a
  sibling submodule (`foo/bar.rs`), add to coverage
  IGNORE_REGEX, then add a `*_TEST_*` env var escape
  hatch in the excluded module so post-call success/
  error branches in the parent module remain
  testable. **Suggested fix:** document this recipe
  in CLAUDE.md alongside the coverage section, or
  add a worked example to the template's reference
  crate.

- **Pedantic clippy allow-list is too minimal for
  realistic codebases.** The template ships only
  `missing_errors_doc`, `missing_panics_doc`,
  `must_use_candidate`, and `module_name_repetitions`
  as allows. kozmotic's existing source -- which is
  not unusual -- needed seven more added on adoption:
  `cast_precision_loss`, `cast_sign_loss`,
  `cast_possible_wrap`, `cast_possible_truncation`,
  `float_cmp`, `struct_field_names`, and
  `too_many_lines`. The numeric casts and float-cmp
  trip on common patterns (test assertions on
  percentages, time arithmetic). **Suggested fix:**
  expand the default allow-list, or split it into a
  "minimal" and "pragmatic" preset and document the
  trade-off.

- **`scripts/` directory is redundant with `xtask`.**
  The template ships seven bash scripts
  (`build.sh`, `test.sh`, `clippy.sh`, `fmt.sh`,
  `validate.sh`, `e2e.sh`, `kill-servers.sh`) that
  duplicate `cargo xtask` subcommands. Migrating
  kozmotic deleted the directory entirely. The
  CLAUDE.md guidance "Never use raw `cargo test`"
  already steers users toward xtask, so the
  scripts/ wrappers serve only e2e and process
  cleanup. **Suggested fix:** drop the build/test/
  clippy/fmt/validate scripts; keep only the e2e and
  kill-servers helpers (since those have web-app-
  specific logic). Or move all of it into xtask and
  drop scripts/ entirely.

- **Stripping the web app is more work than the
  README implies.** The template's "Don't need the
  web app?" section lists six files/directories to
  delete, but doesn't mention the build.ps1 changes
  (Invoke-Dev, Invoke-Frontend, Invoke-E2E with
  port-handling logic), the `frontend` job in CI
  workflow, or the README/llms.txt content that
  references API endpoints and Vite. kozmotic's
  migration touched all of these by hand.
  **Suggested fix:** ship a `cargo xtask
  strip-web` (or a one-shot script) that removes
  every web-related artifact in one pass, including
  workflow jobs and PowerShell command branches.
  Alternative: maintain two template branches
  (`main` for web+CLI, `cli` for CLI-only) so
  projects can pick at clone time.

- **Stop hook running full validate (incl. coverage)
  is slow.** Coverage adds ~15s on a small codebase
  to every Stop hook invocation, on top of compile
  + clippy + tests. For an interactive flow where
  Claude saves often, this compounds. **Suggested
  fix:** offer a fast-path Stop hook variant that
  runs `xtask check + clippy + test` (no coverage)
  and reserve full validate for explicit `/validate`
  or pre-commit. Or sample coverage every Nth Stop.
