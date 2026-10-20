# TODO

Project work queue.

- `cargo xtask todo add --slug <s> --summary "..."` captures an item.
- `/todo` takes the first item, implements it and removes it.
- `/implement <slug>` plans and implements a pending item.
- `/implement` (no arguments) lists pending items and asks
  which to act on.

While an item is being implemented it gets a working planning
doc at `docs/issues/<slug>.md` (problem, plan, decisions). It stays
when the item lands, closed with an `## Outcome` section, as the
record of what shipped -- see the `/implement` skill.

Each item is a headed entry: an `### <slug>` heading, a
`**Summary:**` line, then any prose body. Add and complete items
through `cargo xtask todo`; completing one removes it, and git
history records what shipped. The file is this introduction, then
items only. `list`, `add` and `done` refuse the file when, below the
first item, it holds any other heading (an `## Notes` section, say),
or an `###` heading whose text is not a kebab-case slug (`### Foo
Bar`). The introduction uses `##`. Do not hand-edit it: `todo add` refuses
a body line that would read as a heading, and a hand edit can slip
one in.

### template-sync-remainder

**Summary:** Sync the rest of the rustbase template outside xtask/

kozmotic last synced at rustbase v0.17.0. Its xtask moved onto the published
rustbase-xtask library (breki/rustbase#9), which brought the audit, dep-age,
scripts and clean-cache tooling and the template's validate step list. What
remains is everything outside xtask/, through /template-sync: canon, CI and
config, and the template's current /implement and /retrospect. Known stale on
the way: /implement points to a "Test-Driven Development" section and a web-dev
skill kozmotic lacks; the architect skill's tree puts src/ and tests/ at the
root, not under crates/kozmotic/; llms.txt's subcommand table lacks sessions
prompts.

### adopt-deferred-gates

**Summary:** Adopt the canon, records, doc and deny gates in validate

validate leaves four rustbase-xtask gates out (DEFERRED in
xtask/src/validate.rs); each runs as its own subcommand. One change per gate,
removing it from DEFERRED: Deny needs a deny.toml (the xtask crate already
carries its licence field); Doc needs clean rustdoc links; Canon needs
kozmotic's canon to pass canon-check; Records needs the record files
(docs/todo.md, the reviewer logs, template-feedback.md) in the shape
records-check reads.
