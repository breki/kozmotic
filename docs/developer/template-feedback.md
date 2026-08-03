# Template Feedback

Issues, improvements, and observations about the
[rustbase](https://github.com/breki/rustbase) template
discovered during development of this project.

Use this log to feed improvements back to the template.
Newest entries first.

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
