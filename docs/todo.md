# TODO

Project work queue.

- `/todo <text>` captures a new item with a generated slug.
- `/todo` (no arguments) lists pending slugs.
- `/implement <slug>` plans and implements a pending item.
- `/implement` (no arguments) lists pending items and asks
  which to act on.

While an item is being implemented it gets a working planning
doc at `docs/issues/<slug>.md` (problem, plan, decisions). It is
removed when the item lands, with any durable decision promoted
into the reference docs first -- see the `/implement` skill.

Each item is a headed entry: an `### <slug>` heading, a
`**Summary:**` line, then any prose body. Add and complete items
through `cargo xtask todo`; completing one removes it, and git
history records what shipped. The file is this introduction, then
items only: `list`, `add` and `done` refuse a non-item heading after
the first item, and an `###` heading anywhere that is not an item
(the introduction uses `##`). Do not hand-edit it: `todo add` refuses
a body line that would read as a heading, and a hand edit can slip

### template-sync-remainder

**Summary:** Sync the rest of the rustbase template outside xtask/

kozmotic last synced at rustbase v0.17.0. Its xtask moved onto the published
rustbase-xtask library (breki/rustbase#9), which brought the audit, dep-age and
clean-cache tooling and the template's validate step list; what remains is
everything outside xtask/, through /template-sync: canon, the /implement and
/retrospect skills, CI and config.
