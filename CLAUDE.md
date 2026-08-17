# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code)
when working with code in this repository.

**IMPORTANT: The working directory is already set to the
project root. NEVER use `cd` to the project root or
`git -C <dir>` -- blanket permission rules cannot be
set for commands starting with `cd` or `git -C`, so
they require manual approval every time.**

## Project Overview

Kozmotic is a portable CLI toolkit for AI agents,
written in Rust. Tools emit structured JSON (or human
output via `--format human`) so they compose cleanly
inside Claude Code hooks, status lines, and slash
commands.

- **Stack**: Rust (edition 2024, stable)
- **Target platforms**: Windows, Linux, macOS
- **Output**: structured JSON envelope via `Output<T>`

### Subcommands

| Command | Purpose |
|---------|---------|
| `kozmotic example` | Reference subcommand for new tools |
| `kozmotic agent-ping` | Play notification sounds (presets, files, tones) |
| `kozmotic status-line` | Format Claude Code session JSON for the status bar |
| `kozmotic sessions prompts` | List a session's user prompts from the transcript store |
| `kozmotic self install` | Install the binary into `~/.claude/bin/` |

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `crates/kozmotic` | CLI binary (the toolkit) |
| `xtask` | Build automation |

## Build Commands

```bash
cargo xtask check             # fast compile check
cargo xtask validate          # fmt + clippy + tests + coverage
cargo xtask test [filter]     # tests only
cargo xtask clippy            # lint only
cargo xtask coverage          # coverage only (>=90%)
cargo xtask fmt               # format code
cargo xtask dupes             # code duplication check
cargo xtask changelog add     # add a CHANGELOG entry
cargo xtask todo              # list/add/done TODO items
```

Never use raw `cargo test` or `cargo clippy` -- always
go through `xtask`.

### PowerShell Build Script

```powershell
.\build.ps1 validate    # cargo xtask validate
.\build.ps1 test        # tests only
.\build.ps1 build       # validate + release build
.\build.ps1 clean       # clean artifacts
```

## Coding Standards

- Rust edition 2024
- `#[deny(warnings)]` and `#[forbid(unsafe_code)]` via
  workspace lints
- Clippy pedantic where practical (allow-list in
  workspace `Cargo.toml`)
- Error handling: `thiserror` for typed errors;
  `anyhow` is fine in main if/when added
- Wrap markdown at 80 characters per line
- Maximum code line width: 80 characters (`rustfmt.toml`)

## Development Practices

- **Domain-Driven Design (DDD)**: model around domain
  concepts, not framework primitives.
- **Test-Driven Development (TDD)**: write a failing
  test first; make it pass with the smallest code; then
  refactor. Run `cargo xtask test` after each step.
- **Ask before assuming**: when multiple approaches
  exist, use the `AskUserQuestion` tool to clarify
  rather than guessing.

## Canon vs memory

Two places hold durable guidance, and they are not
interchangeable:

- **Canon** -- this `CLAUDE.md`, `.claude/` skills, commands,
  and agents. Tracked in git, reviewed, shared across machines
  and teammates and fresh clones.
- **Memory** -- per-user auto-memory (e.g.
  `~/.claude/.../memory/`). Per-machine, never committed,
  invisible to everyone else.

**Default to canon.** A rule others would benefit from -- a
workflow convention, a project constraint, a lesson from a
review -- belongs in canon. Reserve memory for genuinely
user-specific items (one operator's preferences, their
role/background, freshly-captured corrections that have not
generalized yet). When a memory entry matures into a shared
rule, promote it to canon and delete the memory copy so the
two do not drift.

## Collaboration

- **Keep responses focused, brief, and concise.** Keep
  disclaimers and caveats short, and spend most of the
  response on the main answer. When asked to explain
  something, give a high-level summary unless an in-depth
  explanation is specifically requested. Use headings,
  tables, and bold only when the content is genuinely a
  list or a comparison; prose does not need scaffolding.
- **Match the length of written documents to what the task
  needs.** Cover the substance, but do not pad with filler
  sections, redundant summaries, or boilerplate. This
  applies to files written to disk -- reports, Markdown
  documents, summaries -- as well as to replies.
- **State things literally.** Do not use a metaphor where a
  plain sentence exists. The reader has to translate the
  image back before learning anything, and the plain version
  is usually shorter. Faults and their fixes:
  "the rule pushes in the same direction the model already
  leans" -> "the rule says to talk more, and the model
  already talks too much";
  "updates only on a real finding" -> "tell the user when
  you find something that changes the work";
  "a rule that corrected an old default can become an
  amplifier of a new one" -> "a rule written to fix an old
  model's weakness can make a new model's excess worse".
  The same fault appears as abstract nouns standing in for
  actions -- "narration", "verification", "a finding". Name
  who does what instead. If a sentence has to be decoded
  before it informs, rewrite it.
- **Write plainly.** One idea per sentence; lead with the
  concrete example, then the rule; prefer plain words
  ("reminder" over "forcing function", "try again" over
  "iterate"); name the subject rather than leaning on "the
  first"/"the latter". Showy phrasing looks crisp but slows
  the reader.
- **Narrate sparingly.** Before the first tool call, say in
  one sentence what you are about to do. After that, give an
  update only when you find something important or change
  direction. When you finish, lead with the outcome: the
  first sentence answers "what happened" or "what did you
  find", with detail after it. Do not announce every step;
  Claude Opus 5 already narrates more than earlier models,
  and an instruction to narrate makes it worse rather than
  better. The original concern -- that a long run of silent
  tool calls reads as "lost" -- is covered by the opening
  sentence and the updates on real findings.
- **Lead with context before a decision-making question, and
  show concrete artifacts** -- for a technical choice (CLI
  shape, JSON schema, data layout), write out what each
  option looks like (side-by-side snippets / diffs) *before*
  the `AskUserQuestion`. Option labels summarize choices the
  user has already seen, not the first encounter.
- **`AskUserQuestion`: explain in layman's terms, short.**
  The lead prose must be readable by a non-expert: no
  internal type names, file paths, or API names in the
  problem statement (save those for the option
  descriptions). It states *what the decision means*, not
  *how it is implemented*.

## Environment Constraints

Machine-level assumptions, so the assistant does not reach
for tools that are not present:

- The project builds with stable Rust via `rustup`/`mise`.
  Prefer `cargo xtask` for scripting over shelling out to
  Python or Node, neither of which is a project dependency.
- `cargo-llvm-cov` is required by the coverage gate and
  `code-dupes` by `cargo xtask dupes`.
- Audio playback (`agent-ping`) needs a real output device;
  `KOZMOTIC_TEST_AUDIO` short-circuits it in tests.

## Adding a new tool

1. Add a variant to `Commands` in
   `crates/kozmotic/src/main.rs`.
2. Implement the handler in its own module under
   `crates/kozmotic/src/`.
3. Wrap the result with `Output::success(tool, data)`.
4. Respect the `--format` flag for JSON vs human
   output.
5. Cover the new path in
   `crates/kozmotic/tests/integration_test.rs` using
   `assert_cmd`.

## Commits

**All commits must go through the `/commit` skill.**
Never use `git commit` directly. No `Co-Authored-By`,
no emoji.

Conventional Commits format with an AI-generated footer:

```
type(scope): subject

Body explaining what and why (wrapped at 72).

AI-Generated: Claude Code (<ModelName> <Date>)
```

- Header: 50 chars max, imperative mood, no trailing
  period.
- Body: 72-char wrap, focuses on what/why.
- Footer: `AI-Generated: Claude Code (<ModelName>
  <Date>)`. No `Co-Authored-By`. No
  `Generated with Claude Code` lines.

## Definition of Done

A task is done only when all of the following hold -- not
just when the code compiles:

1. **Targeted tests** for the change are written and pass.
2. **Type-check** passes (`cargo xtask check`).
3. **Self-review the diff** before committing.
4. **Code review** -- the `red-team` and `artisan` agents
   have run over the staged diff (see `/commit` step 3).
5. **`cargo xtask validate`** passes (the umbrella gate).

`cargo xtask validate` checks:

1. **Formatting**: `cargo fmt --all -- --check`
2. **No warnings**:
   `cargo clippy --all-targets -- -D warnings`
3. **All tests pass**: `cargo test`
4. **Coverage >= 90%** (per-module floor 85%)

`cargo xtask dupes` (duplication <= 6%, production code
only) is run separately rather than as a validate step.

## Semantic Versioning

Follow [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** -- breaking changes to CLI interface or
  JSON output schema
- **MINOR** -- new subcommands, flags, or backwards-
  compatible features
- **PATCH** -- bug fixes, documentation, internal
  refactors

The version lives in `crates/kozmotic/Cargo.toml` and
is the **single source of truth**.

## Release Notes

Maintain `CHANGELOG.md` using the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
format. Group changes under: **Added**, **Changed**,
**Fixed**, **Removed**.

Always keep an `[Unreleased]` section at the top.

## Planning

`TODO.md` tracks upcoming tasks under `## Pending` and
`## Done`. Check it before starting new work and keep it up
to date as items are completed or added.

Edit it through `cargo xtask todo` rather than by hand -- the
sections sit far apart and a hand edit easily splits one with
a duplicate heading:

```bash
cargo xtask todo list                       # pending slugs
cargo xtask todo add --slug <s> --summary "..."
cargo xtask todo done --slug <s>
```

The same applies to `CHANGELOG.md`:

```bash
cargo xtask changelog add --kind added "..."   # or changed/fixed/removed
```

## Workspace lints and xtask overrides

The workspace forbids `unsafe_code` via
`[workspace.lints.rust]` so production crates inherit the
policy by default. If `xtask/` ever needs OS-specific code
(for example Win32 process APIs), redefine the lints block
locally for `xtask` only rather than weakening the workspace
policy:

```toml
# xtask/Cargo.toml
[lints.rust]
warnings = "deny"
unsafe_code = "allow"   # xtask is build tooling, scoped exception
```

Production crates keep `[lints] workspace = true` and remain
`unsafe`-forbidden. Document the scoped exception with a
comment near the use site so reviewers can verify the unsafe
block is genuinely necessary.

## Coverage exceptions for hardware-bound code

The 90% coverage gate assumes every code path can run under
`cargo llvm-cov` in CI. Some cannot: audio playback, network
calls against external services, native API calls. The
recipe for keeping the gate honest without weakening it:

1. **Extract the hardware-bound code into a sibling
   submodule.** Given `foo.rs` holding both business logic
   and an I/O call, split into `foo.rs` (the orchestrator)
   and `foo/bar.rs` (the I/O leaf). The leaf should be as
   small as possible -- ideally just the unmockable call and
   its immediate error mapping. This project does exactly
   that with `agent_ping/playback.rs` and
   `status_line/api_status/io.rs`.
2. **Exclude the leaf via manifest config.** Add its path (a
   regex fragment) to `[workspace.metadata.coverage] ignore`
   in the **root `Cargo.toml`** -- no need to fork `xtask`:

   ```toml
   [workspace.metadata.coverage]
   # Single-quoted TOML literal strings, so backslashes reach
   # the regex verbatim (no doubling).
   ignore = ['agent_ping[/\\]playback\.rs$']
   ```

   `cargo xtask coverage` merges these with its baseline
   (`src/main.rs`, `src/bin/`); the leaf is exempt, the
   orchestrator is not. An absent section leaves the baseline
   unchanged, and a missing manifest degrades to the baseline
   rather than failing. A pattern matching *every* file
   (empty, `.`, `.*`, `.+`) is rejected -- it would silently
   neuter the gate. Only the `[workspace.metadata.coverage]`
   header plus a line-leading `ignore = [...]` is read; the
   dotted-key and inline-table spellings are not.
3. **Add a `KOZMOTIC_TEST_*` escape hatch in the excluded
   module.** `KOZMOTIC_TEST_AUDIO` short-circuits the real
   native call and returns a fixed `Ok`/`Err`, keeping the
   parent module's success and error branches testable --
   they carry the business logic and stay inside the gate.

When NOT to use this recipe: if the I/O can be faked with a
trait plus dependency injection at the call site without
contortions, do that instead.

## Lints: `doc_markdown` allowlist via `clippy.toml`

Clippy runs with pedantic lints enabled where practical.
`clippy::doc_markdown` flags identifiers like `PowerShell`,
`JSON`, `macOS`, `GitHub` in doc comments, forcing every
occurrence to be backticked even when the prose reads
naturally without them.

`clippy.toml` at the workspace root carries a curated
`doc-valid-idents` allowlist. It extends clippy's defaults
(via the `".."` sentinel as the first entry) rather than
replacing them. **Append** new domain terms to the kozmotic
block at the end of that list rather than redefining it.

## Long-running scripts

For any script that runs more than ~30 seconds:

- **Author side** -- tee stdout to `target/<name>.log` so
  output is durable. With the `exec > >(tee "$LOG") 2>&1`
  idiom you must also capture `TEE_PID=$!` and
  `wait "$TEE_PID"` in the `EXIT` trap -- bash does not
  synchronize with `>(...)` process substitution on exit, so
  trailing trap output is silently truncated without it.
- **Caller side** -- **never pipe a long-running command
  through `tail -N` under a tight timeout.** `tail -N` says
  "give me the end"; the timeout says "there will be no
  end" -- it buffers until an EOF that never arrives, so the
  pipeline shows nothing and reads as a stall. Use
  `run_in_background` for completion, or a `Monitor` with a
  line-buffered grep for progress; reserve `| tail -N` for
  already-finished commands.

## Edition-2024 migration notes

The project is on Rust edition 2024. Upgrading from an older
snapshot routinely hits a small set of mechanical fixes that
`cargo fix --edition` either applies or flags:

- **Unsafe extern blocks**: `extern "C" { fn foo(); }` must
  become `unsafe extern "C" { fn foo(); }`.
- **Match ergonomics tightening**: bare `ref` patterns inside
  a binding that already implies a reference must be dropped.
- **`gen` is reserved**: any identifier called `gen` needs
  the raw-identifier form `r#gen` or a rename.
- **Nested `if let` -> let chains**: clippy's autofix
  collapses `if x { if y { ... } }` into `if x && y { ... }`.

Run `cargo fix --edition --workspace` followed by
`cargo xtask validate`.

## Skills

| Skill | Purpose |
|-------|---------|
| `/check` | Fast compilation check (no tests) |
| `/test` | Run tests with agent-friendly output |
| `/validate` | Full quality pipeline with stepwise progress |
| `/commit` | Commit with versioning, diary, and conventions |
| `/release` | Prepare a versioned release |
| `/todo` | Add a TODO item or implement the next pending one |
| `/simplify` | Review changed code for quality |
| `/architect` | Project overview and architecture guide |
| `/agent-cli` | Patterns for agent-friendly CLI subcommands |
| `/sound` | Toggle hook sounds on/off |
| `/statusline-setup` | Configure status line widgets |
| `/template-improve` | Log feedback for the rustbase template |
| `/template-sync` | Sync upstream template changes |
| `/implement` | Plan and implement a pending TODO item |
| `/retrospect` | Workflow retrospective on the session |
| `/html-report` | Build a self-contained local HTML report |

### Reviewer agents

Two read-only agents guard every code commit, spawned in
parallel by `/commit` (step 3):

| Agent | Focus | Tools |
|-------|-------|-------|
| `red-team` | Security & correctness, adversarial | `Read, Grep, Glob, Bash` |
| `artisan` | Code quality & craftsmanship beyond clippy | `Read, Grep, Glob` |

Gating rules live in `.claude/commands/code-reviewers.md`;
the review criteria live in `.claude/agents/`. Deferred
findings are logged in `docs/developer/redteam-log.md` and
`docs/developer/artisan-log.md`.

## Template Sync

This project tracks its template origin in
`.template-sync.toml`. Use `/template-sync` to pull
improvements from the upstream
[rustbase](https://github.com/breki/rustbase) template.
The command fetches upstream changes, categorizes
them, and helps you selectively apply relevant updates
while preserving your project's customizations.

## Template Feedback

This project was generated from the
[rustbase](https://github.com/breki/rustbase) template.
When you notice anything in the template-provided
files that is suboptimal, incorrect, outdated, or
could be improved, log it in
`docs/developer/template-feedback.md`.

## Tone reminder

Repeated at the end deliberately: in a long prompt, a
conciseness instruction near the top fades. Claude Opus 5
defaults to longer responses, more progress narration, and
longer written deliverables than earlier models. Anthropic's
Opus 5 prompting guide notes that lowering `effort` reduces
thinking but not visible output, so length has to be asked
for explicitly.

<tone_preference>
Keep outputs reasonably concise. Lead with the answer.
Structure only what is actually a list.
</tone_preference>
