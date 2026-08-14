# Development Diary

This diary tracks functional and infrastructure
changes to the kozmotic codebase in reverse
chronological order. Only record changes that are
significant or non-obvious — routine bug fixes and
small tweaks don't need entries.

---

### 2026-08-14

- Release reach: ARM Linux in, musl out

    Added an `aarch64-unknown-linux-gnu` artifact on
    GitHub's ARM runner and pinned the Linux builds to
    `ubuntu-22.04`, since the build runner's glibc is
    the minimum glibc of every machine that can run
    the artifact (2.39 → 2.35).

    A static `x86_64-unknown-linux-musl` build was
    attempted and abandoned: `alsa-sys` needs a musl
    build of alsa-lib, which no distribution ships, so
    the build fails at its custom build script even
    with `musl-tools` present. Making it work would
    mean cross-compiling alsa-lib or feature-gating
    `rodio` out of the binary — and the result would
    still need `libasound` at runtime, defeating the
    point of static linking. Note that ALSA is a hard
    runtime dependency of the whole binary, not just
    `agent-ping`: without `libasound.so.2` even
    `status-line` fails to start. That is now
    documented in the README.

- Host widgets and an api-status that never hides

    Added `host`, `ram`, and `disk` status-line
    widgets, backed by a new `sysinfo` dependency.
    Hand-rolling this was rejected: cross-platform
    free-space queries need `statvfs` /
    `GetDiskFreeSpaceEx`, and the workspace forbids
    unsafe code, so the alternative was shelling out
    to `df`/`wmic`. `disk` resolves the mount by
    longest path-prefix match on the session's
    working directory, which makes it report the
    volume the user is actually filling up.

    `api-status` used to return `Option<String>` and
    vanish whenever the fetch failed — during the
    outage of 2026-08-14 it showed nothing, which is
    visually identical to a healthy API. The domain
    type is now an `ApiHealth` enum with no empty
    case (`Current` / `Stale` / `Unknown`). The
    deeper cause was the missing timeout: an
    unresponsive status page blocked the render until
    Claude Code gave up on the command. Requests are
    now bounded (1.5s connect, 2.5s total) and the
    cache records failed attempts so an outage is
    retried at most every 30 seconds instead of on
    every prompt.

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
