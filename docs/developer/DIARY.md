# Development Diary

This diary tracks functional and infrastructure
changes to the kozmotic codebase in reverse
chronological order. Only record changes that are
significant or non-obvious — routine bug fixes and
small tweaks don't need entries.

---

### 2026-03-25

- Initial project setup and restructuring

    Extracted single-file `src/main.rs` into modules:
    `output`, `agent_ping`, `self_install`. Added
    `xtask` workspace crate with `validate` command
    (fmt + clippy + test). Replaced BigSoundBank
    sounds with subtle Pixabay chimes for Stop,
    StopFailure, and Notification presets. Wired
    agent-ping into Claude Code hooks for audible
    feedback on stop, error, and notification events.

    Imported development practices from the
    ledgerstone project: clippy lint config in
    `Cargo.toml`, cargo aliases via
    `.cargo/config.toml`, and the xtask pattern for
    build automation.
