# Artisan Findings -- Deferred backlog

Artisan (code-quality) findings that were **deferred** --
real, but not fixed at review time. Fixed findings are not
logged here; their resolution lives in the commit that fixed
them.

Newest first; add new entries right after the `---`. Use a
self-describing ID `aq-<YYYY-MM-DD>-<kebab-slug>` (no central
counter); a later commit acting on an item cites the ID
inline. Each entry: the ID heading, a `**Category:**` line,
and a short description.

**Threshold:** when 10+ items are open here, a full-codebase
Artisan review is warranted before continuing feature work.

---

## aq-2026-08-18-split-git-module

**Category:** Module size / separation of concerns

`crates/kozmotic/src/status_line/git.rs` is 590 lines and
holds three separable concerns: parsing git's textual output
(`parse_file_counts`, `parse_ahead_behind`,
`count_numstat_files`, `sum_numstat_lines`), deciding what to
say (`render_file_counts`, `ahead_text`, `classify_sync`,
`render_diff_lines`, `render_status_counts`), and spawning
processes behind a per-render cache (`GitContext`, `run_git`).

Proposed split: `git/parse.rs`, `git/render.rs`, and `git.rs`
keeping `GitContext`, `run_git` and the `render` dispatcher.
That also isolates the process-spawning code, which is the
part that resists coverage.

Deferred because the split is a refactor of ~400 existing
lines with no behaviour change, well outside the diff that
surfaced it (the `git-ahead` no-upstream change).
