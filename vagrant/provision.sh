#!/usr/bin/env bash
#
# Provisions the kozmotic agent VM. Runs on first boot as the
# `vagrant` user, and again on every `bombyx provision`.
#
# Re-runnable by construction. Two rules keep it that way, and
# both were learned by breaking them:
#
#   1. Guard on the finished artefact, not on a step having
#      started. A guard that sees a half-made file and skips the
#      work leaves the VM wedged.
#   2. A step that can fail must be able to fail the script.
#      Command substitutions inside an argument, and pipelines,
#      both swallow exit status -- see the notes at each use.

set -euo pipefail

# --------------------------------------------------------------
# Pinned versions, and the hashes that go with them
# --------------------------------------------------------------
# Every binary this script downloads is checked against a hash
# recorded here. That matters more than it looks: without it, the
# script trusts whatever the download host serves today. With it,
# the host has to serve the same bytes that were reviewed when the
# pin was added.
#
# The honest limit: each hash was read from the same origin that
# serves the binary, once. This is trust on first use, not a
# signature chain. What it buys is that the trust decision
# happened once, visibly, in a reviewed edit -- instead of
# silently on every provision.
#
# **Change a version and you must change its hash.** A stale hash
# stops the build, which is the point; the error message says
# which constant to update.
#
# Claude Code is the exception. Its hash covers the artifact the
# VM is bootstrapped from, and nothing after that: the tool
# updates itself and is left to. See the Claude Code section for
# why that trade was taken.
#
# The cargo-installed tools further down are the other exception,
# and deliberately so -- see that section.

SWAP_GB=2

# Supplied by the Vagrantfile from vagrant/local.env, which is
# per-developer and gitignored. Empty is a supported state: the
# git identity is then not set, and provisioning says so.
GIT_USER_NAME="${GIT_USER_NAME:-}"
GIT_USER_EMAIL="${GIT_USER_EMAIL:-}"

RUSTUP_VERSION="1.29.0"
RUSTUP_SHA256="4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10"

# The version a fresh VM is bootstrapped from. It will not stay on
# it: Claude Code updates itself, on purpose. Bump this only to
# move the starting point, and change the hash with it.
CLAUDE_VERSION="2.1.226"
CLAUDE_SHA256="4e9bec1177ce9690e8bd988b710ac24105e70da428dd094c5adcbbe786a55555"

# kozmotic is installed here as a *tool* -- it draws the Claude
# Code status line -- not as the thing under development. The
# agent's own working copy is the checkout further down, and
# building that is the agent's job.
#
# Installing a release binary rather than building the checkout
# also keeps the status line working while the checkout is mid-
# refactor and does not compile.
KOZMOTIC_VERSION="v1.1.0"
KOZMOTIC_SHA256="6af414179a88abbd9cb533e4033cb5f3bf0d04d6db6401faa18d0dbddfd1c983"

# --------------------------------------------------------------
# Helpers and preconditions
# --------------------------------------------------------------

log() { printf '\n== %s\n' "$1"; }

# Note for anyone porting this from jutro's copy: that one calls
# `warn` without defining it. Under `set -euo pipefail` an unknown
# command exits 127, so a VM provisioned with no vagrant/local.env
# fails outright at the git-identity step -- the one step whose
# comment promises it degrades gracefully.
warn() { printf '\n!! %s\n' "$*" >&2; }

fail() {
  echo "$*" >&2
  exit 1
}

# Verifies a file against a pinned hash, or stops the run.
verify_sha256() {
  local file="$1" expected="$2" label="$3" actual
  [ -f "$file" ] || fail "$label: expected a file at $file"
  actual="$(sha256sum "$file" | cut -d' ' -f1)"
  if [ "$actual" != "$expected" ]; then
    echo "$label: checksum mismatch -- refusing to continue" >&2
    echo "  file:     $file" >&2
    echo "  expected: $expected" >&2
    echo "  actual:   $actual" >&2
    echo "If you changed a pinned version, update the matching" >&2
    echo "*_SHA256 near the top of this script." >&2
    exit 1
  fi
}

# The pinned hashes are all for x86_64 builds. On any other
# architecture they would fail confusingly halfway through, so say
# so up front.
ARCH="$(uname -m)"
[ "$ARCH" = "x86_64" ] ||
  fail "this script pins x86_64 builds; this VM reports $ARCH"

# One workspace for the whole run, removed however the script
# exits.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Set here so the checks below can find what was just installed.
# The files that make this stick for future shells are written
# further down.
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

# --------------------------------------------------------------
# Login shell
# --------------------------------------------------------------
# The box creates this user with /bin/sh, which on Debian is dash.
# Dash has no line editing, so an interactive session gets no
# history and the arrow keys echo their escape sequences (`^[[A`)
# instead of recalling commands. The cause is not obvious from the
# symptom: TERM and terminfo are both fine, because dash never
# consults them.
#
# Set here rather than by hand so it survives a destroy and
# rebuild. Takes effect on the next connection, not this one.
WHOAMI="$(id -un)"
CURRENT_SHELL="$(getent passwd "$WHOAMI" | cut -d: -f7)"
if [ "$CURRENT_SHELL" != "/bin/bash" ]; then
  log "setting login shell to bash (was $CURRENT_SHELL)"
  sudo chsh -s /bin/bash "$WHOAMI"
fi

# --------------------------------------------------------------
# Swap
# --------------------------------------------------------------
# The VM has 4 GB and `rustc` linking a workspace is the step that
# will exhaust it. Swap turns an out-of-memory kill during a link
# into a slow link, which is much easier to diagnose -- an
# OOM-killed linker usually surfaces as a bare "signal: 9", with
# nothing naming memory as the cause.
#
# Two traps are avoided here, both of which bit earlier versions:
#
# `swapon` is run through sudo even to *read* state, because it
# lives in /usr/sbin, which is not on the PATH a non-interactive
# shell gives an unprivileged user. A bare `swapon --show` fails
# with "command not found", which reads as "no swap configured".
#
# The output is captured and matched with `case` rather than piped
# into `grep -q`. Under `set -o pipefail`, `grep -q` exits at the
# first match and closes the pipe, so a producer with more to
# write dies of SIGPIPE and the pipeline reports failure even
# though the pattern *was* found.
SWAP_STATE="$(sudo swapon --show 2>/dev/null || true)"
case "$SWAP_STATE" in
*/swapfile*) ;;
*)
  # `-e /swapfile` is not a safe guard: `fallocate` creates the
  # file and `mkswap` writes its signature, so an interrupt
  # between them leaves a file that exists and has no signature.
  # Every later run would then skip creation and fail on
  # `swapon: read swap header failed`, permanently. Asking
  # `swaplabel` whether it is really a swap area is the honest
  # question, and the build-then-rename below means /swapfile
  # never exists in a half-made state.
  if ! sudo swaplabel /swapfile >/dev/null 2>&1; then
    log "creating ${SWAP_GB}G swapfile"
    sudo rm -f /swapfile /swapfile.new
    sudo fallocate -l "${SWAP_GB}G" /swapfile.new
    sudo chmod 600 /swapfile.new
    sudo mkswap /swapfile.new
    sudo mv /swapfile.new /swapfile
  fi
  log "enabling swapfile"
  sudo swapon /swapfile
  ;;
esac

if ! grep -qF '/swapfile' /etc/fstab; then
  echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab >/dev/null
fi

# --------------------------------------------------------------
# System packages
# --------------------------------------------------------------
# Kept to what building kozmotic needs: a C toolchain and linker
# for Rust's `-sys` crates, pkg-config and OpenSSL headers, plus
# git, curl, jq and ca-certificates.
#
# Node is deliberately absent. jutro's VM installs it for a
# frontend and a Playwright suite; kozmotic is a pure Rust CLI
# with no JavaScript anywhere, and Claude Code is installed from
# its own native release below rather than through npm. Nothing
# left needs a Node runtime.
log "installing system packages"
sudo apt-get update -qq
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
  build-essential \
  pkg-config \
  libssl-dev \
  git \
  curl \
  jq \
  ca-certificates

# ALSA development headers, needed to *build* kozmotic.
#
# This is the one system dependency specific to this project, and
# it differs from jutro's VM in a way worth stating: jutro only
# installs the ALSA *runtime*, because it downloads a prebuilt
# kozmotic binary and merely has to load libasound.so.2. Here the
# agent compiles kozmotic from source, and its `rodio` dependency
# (with the `playback` feature) pulls in `cpal` -> `alsa-sys`,
# whose build script asks pkg-config for `alsa`. Without the -dev
# package that lookup fails and `cargo build` stops with an
# alsa-sys build error naming pkg-config -- which reads like a
# broken pkg-config rather than a missing library.
#
# The -dev package depends on the runtime, so installing it covers
# both the compile and the resulting binary.
#
# Asking pkg-config is the honest guard: it is the same question
# the build script asks. Checking `ldconfig -p` for the runtime
# would pass on a machine that can load kozmotic but cannot build
# it.
if ! pkg-config --exists alsa 2>/dev/null; then
  log "installing ALSA development headers (needed to build rodio)"
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    libasound2-dev
fi

pkg-config --exists alsa ||
  fail "libasound2-dev installed but pkg-config still cannot find alsa"

# --------------------------------------------------------------
# Rust
# --------------------------------------------------------------
# Installed per-user via rustup rather than from apt: kozmotic
# pins its toolchain in rust-toolchain.toml, and rustup is what
# honours that file. A distro-packaged rustc would ignore it.
#
# `rustup-init` is downloaded from the versioned archive and
# checked against the pinned hash before it runs. The usual
# `curl https://sh.rustup.rs | sh` executes whatever arrives, and
# an installer that runs before any verification is a gap the
# later checks cannot close.
if [ ! -x "$HOME/.cargo/bin/rustup" ]; then
  log "installing rustup ${RUSTUP_VERSION}"
  curl --proto '=https' --tlsv1.2 -fsSL -o "$WORK/rustup-init" \
    "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init"
  verify_sha256 "$WORK/rustup-init" "$RUSTUP_SHA256" "rustup-init"
  chmod +x "$WORK/rustup-init"
  "$WORK/rustup-init" -y --no-modify-path --default-toolchain stable
fi

# Run as a statement, not inside the `log` argument below. A
# command substitution used as an argument throws its exit status
# away -- `set -e` sees only `log` succeeding -- so a missing
# toolchain would print an empty version and the run would report
# success.
rustc --version >/dev/null || fail "rustup installed but rustc will not run"
log "rust: $(rustc --version)"

# rust-toolchain.toml asks for clippy and rustfmt, which rustup
# fetches on first use inside the checkout. llvm-tools is not in
# that file and is not fetched for you: `cargo llvm-cov` needs the
# profiling runtime it provides, and without it the coverage gate
# fails with a message about a missing `llvm-profdata` rather than
# about a missing component.
if ! rustup component list --installed | grep -q '^llvm-tools'; then
  log "adding the llvm-tools component (needed by cargo-llvm-cov)"
  rustup component add llvm-tools-preview
fi

# --------------------------------------------------------------
# The tools `cargo xtask` shells out to
# --------------------------------------------------------------
# `cargo xtask validate` is the project's own acceptance gate, and
# two of its steps are separate binaries: coverage runs
# `cargo llvm-cov` and the duplication check runs `code-dupes`.
# Neither ships with Rust, so without this section the agent's
# first `validate` fails on tooling rather than on its own work.
#
# These two are installed from crates.io rather than downloaded
# and hash-pinned like everything else above, which is a
# deliberate exception. crates.io is already a trust root for this
# VM -- every dependency kozmotic compiles comes from there -- so
# `cargo install` adds no new party to trust, whereas a pinned
# tarball would add two more hashes to keep current. `--locked`
# makes each build use the lockfile the crate was published with
# instead of resolving fresh versions today.
#
# The cost is real: these compile from source, which on 2 vCPUs is
# the slowest part of a first provision by a wide margin. They are
# guarded on the binary being present, so it is a first-boot cost
# only.
for tool in cargo-llvm-cov code-dupes; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    log "installing $tool from crates.io (this takes a while)"
    cargo install --locked "$tool"
  fi
done

cargo llvm-cov --version >/dev/null ||
  fail "cargo-llvm-cov installed but will not run"
code-dupes --version >/dev/null ||
  fail "code-dupes installed but will not run"
log "cargo tools: $(cargo llvm-cov --version), $(code-dupes --version)"

# --------------------------------------------------------------
# Claude Code
# --------------------------------------------------------------
# The binary is downloaded straight from the releases URL and
# verified against the pinned hash before anything executes.
#
# Not `npm install -g`: that package's postinstall would run as
# root, it only bootstraps this same native install anyway, and
# this VM has no Node at all.
#
# Not `curl https://claude.ai/install.sh | bash` either. That
# script does verify what it downloads, but nothing verifies the
# script, and it runs first. A tampered installer could place the
# correctly-hashed binary -- passing every check below -- while
# also writing a wrapper earlier on PATH. Fetching the binary
# ourselves removes that step entirely.
#
# CLAUDE_VERSION is the version this VM is *bootstrapped* from,
# not the version it will keep running. Claude Code updates
# itself, and that is left alone on purpose. Fighting a
# self-updating tool is a losing battle, and the pin was not
# buying much: a checksum proves the artifact you bootstrap from
# is the one you reviewed, but it cannot protect against a future
# release, since a bump just moves the pin. In a disposable VM
# whose real protection is containment, running a stale agent is
# the bigger cost.
#
# So the guard is "is claude installed at all", not "is the pinned
# version installed": the download is verified once, on a VM that
# has none, and after that the tool manages itself.
#
# No credentials are configured, deliberately: this VM exists so
# that an agent has nothing to steal. Sign in inside the VM, or
# export ANTHROPIC_API_KEY there yourself. A key baked in here
# would live in the repo and in every archive bombyx pushes.
if ! command -v claude >/dev/null 2>&1; then
  log "installing claude code ${CLAUDE_VERSION}"
  curl -fsSL -o "$WORK/claude" \
    "https://downloads.claude.ai/claude-code-releases/${CLAUDE_VERSION}/linux-x64/claude"
  verify_sha256 "$WORK/claude" "$CLAUDE_SHA256" "claude code (download)"
  chmod +x "$WORK/claude"
  "$WORK/claude" install "$CLAUDE_VERSION"
fi

claude --version >/dev/null || fail "claude was installed but will not run"
log "claude: $(claude --version)"

# --------------------------------------------------------------
# kozmotic (status line)
# --------------------------------------------------------------
# A prebuilt release binary rather than `cargo install --path`
# from the checkout: the linux-gnu asset matches this VM exactly,
# and downloading it takes a second where building takes minutes
# on 2 vCPUs. The repository is public, so no credential is
# needed.
#
# The guard compares the installed binary's hash rather than
# asking whether the file exists. A git tag is mutable and the
# release asset behind it can be replaced, so "a file is there"
# proves nothing about which build it is -- and guarding on
# existence alone meant a version bump was silently ignored on any
# VM that already had the old one.
#
# Note that the agent may replace this binary itself: `kozmotic
# self install` writes to exactly this path. That is expected --
# testing a locally built kozmotic as the live status line is a
# reasonable thing to do in this VM -- and the next `bombyx
# provision` puts the pinned release back.
KOZMOTIC_BIN="$HOME/.claude/bin/kozmotic"
KOZMOTIC_CURRENT=""
if [ -f "$KOZMOTIC_BIN" ]; then
  KOZMOTIC_CURRENT="$(sha256sum "$KOZMOTIC_BIN" | cut -d' ' -f1)"
fi

if [ "$KOZMOTIC_CURRENT" != "$KOZMOTIC_SHA256" ]; then
  log "installing kozmotic ${KOZMOTIC_VERSION}"
  mkdir -p "$HOME/.claude/bin" "$WORK/kozmotic"
  KOZMOTIC_TAR="kozmotic-${KOZMOTIC_VERSION}-x86_64-unknown-linux-gnu.tar.gz"
  curl -fsSL -o "$WORK/$KOZMOTIC_TAR" \
    "https://github.com/breki/kozmotic/releases/download/${KOZMOTIC_VERSION}/${KOZMOTIC_TAR}"
  tar -xzf "$WORK/$KOZMOTIC_TAR" -C "$WORK/kozmotic" --no-same-owner

  # The archive holds a versioned directory, not a bare binary.
  # Assigned and checked rather than piped into `xargs`: a
  # pipeline cannot fail this script, so a `find` that matched
  # nothing would install nothing and still report success.
  KOZMOTIC_SRC="$(find "$WORK/kozmotic" -type f -name kozmotic -print -quit)"
  [ -n "$KOZMOTIC_SRC" ] || fail "no kozmotic binary inside $KOZMOTIC_TAR"

  verify_sha256 "$KOZMOTIC_SRC" "$KOZMOTIC_SHA256" "kozmotic (download)"
  install -m 0755 "$KOZMOTIC_SRC" "$KOZMOTIC_BIN"
fi

verify_sha256 "$KOZMOTIC_BIN" "$KOZMOTIC_SHA256" "kozmotic (installed)"

# Runs the binary rather than testing that the file exists. It
# links against libasound, and on an image without that library it
# installs cleanly and then fails to start with "error while
# loading shared libraries: libasound.so.2". A check that only
# looked for the file would call that a success.
if ! "$KOZMOTIC_BIN" --version >/dev/null 2>&1; then
  echo "kozmotic installed but will not run:" >&2
  "$KOZMOTIC_BIN" --version >&2 || true
  exit 1
fi

log "kozmotic: $("$KOZMOTIC_BIN" --version), checksum verified"

# --------------------------------------------------------------
# PATH for future shells
# --------------------------------------------------------------
# Both files are written, and the order matters in one of them.
#
# Debian's ~/.bashrc opens with `case $- in *i*) ;; *) return;;
# esac`, which returns immediately for a non-interactive shell.
# Anything appended to the end of that file is therefore
# unreachable for `ssh vm 'claude --version'` and anything else
# driven remotely -- which is exactly the case a PATH entry is
# needed for. So the line is prepended, above that check.
#
# ~/.profile has no such guard but is read only by login shells,
# which is why it is not sufficient on its own.
PATH_MARKER='# provisioned by vagrant/provision.sh'
PATH_LINE='export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"'

touch "$HOME/.profile" "$HOME/.bashrc"

if ! grep -qF "$PATH_MARKER" "$HOME/.profile"; then
  printf '%s\n%s\n' "$PATH_MARKER" "$PATH_LINE" >> "$HOME/.profile"
fi

if ! grep -qF "$PATH_MARKER" "$HOME/.bashrc"; then
  {
    printf '%s\n%s\n\n' "$PATH_MARKER" "$PATH_LINE"
    cat "$HOME/.bashrc"
  } > "$WORK/bashrc"
  # Copied rather than moved, so the file keeps its own ownership
  # and permissions rather than the temp file's.
  cat "$WORK/bashrc" > "$HOME/.bashrc"
fi

# --------------------------------------------------------------
# Claude Code settings
# --------------------------------------------------------------
# Written on every run: provisioning is the source of truth for
# this VM, so an edit made inside the VM is not preserved.
#
# The status line and `effortLevel` match the workstation's, so
# an agent in here behaves the way it does outside. Setting the
# effort in this file rather than by typing `/effort` in the VM
# is the point: a session started by a script, or by anyone who
# does not know to set it, gets the same level, and a destroy and
# rebuild does not quietly lose it.
#
# The workstation's sound hooks are deliberately left out -- this
# VM has no audio device, so they would fail on every stop with
# nothing to play.
# That is also why `kozmotic agent-ping` cannot be smoke-tested
# here for real playback; its own tests cover the logic through
# `--dry-run` and the KOZMOTIC_TEST_AUDIO override, neither of
# which touches a device, so `cargo xtask validate` passes on a
# silent VM.
log "writing claude code settings"
mkdir -p "$HOME/.claude"
cat > "$HOME/.claude/settings.json" <<'JSON'
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/bin/kozmotic status-line --show 'directory,git-branch,git-ahead,git-files,git-lines,last-commit;rate-limit,rate-limit-7d,cost-rate,duration;context,lines,api-status'"
  },
  "effortLevel": "medium",
  "theme": "dark",
  "editorMode": "normal",
  "feedbackSurveyRate": 0
}
JSON

# --------------------------------------------------------------
# Claude Code first-run state
# --------------------------------------------------------------
# ~/.claude.json is a different kind of file from settings.json
# above, and the difference decides how it is written.
# settings.json is configuration this script owns, so it is
# rewritten wholesale every run. ~/.claude.json is *runtime state*
# -- Claude Code keeps session history and per-project records
# there -- so specific keys are merged into it and everything else
# is left alone.
#
# Seeding it only when absent does not work, which is worth
# recording because it looks like it should: `claude install` and
# the `claude --version` check above both create the file, so by
# the time this runs it always exists, even on a brand-new VM. A
# merge is the only form that works on both a fresh box and one
# that has been used.
#
# What the merge buys: the onboarding flow and the per-directory
# "do you trust the files in this folder?" prompt are skipped, so
# a fresh VM drops straight into a usable session instead of a
# wizard. Trust is pre-granted only for the home directory and the
# expected checkout, not blanket-enabled.
#
# Nothing identifying is copied from the workstation; Claude Code
# mints its own IDs.
log "configuring claude code first-run state"
[ -f "$HOME/.claude.json" ] || echo '{}' > "$HOME/.claude.json"

jq --arg v "$CLAUDE_VERSION" --arg home "$HOME" '
    .hasCompletedOnboarding = true
  | .lastOnboardingVersion = $v
  | .lastReleaseNotesSeen = $v
  | .projects[$home].hasTrustDialogAccepted = true
  | .projects[$home].hasCompletedProjectOnboarding = true
  | .projects[$home + "/kozmotic"].hasTrustDialogAccepted = true
  | .projects[$home + "/kozmotic"].hasCompletedProjectOnboarding = true
' "$HOME/.claude.json" > "$WORK/claude.json"

# Copied back rather than moved, so the file keeps its own
# permissions -- Claude Code creates it 0600 and it should stay
# that way.
cat "$WORK/claude.json" > "$HOME/.claude.json"

# --------------------------------------------------------------
# The kozmotic repository
# --------------------------------------------------------------
# kozmotic is public, so the clone needs no credential and the VM
# is useful without one: the agent can read, build, test and
# commit locally. This is the main structural difference from
# jutro's VM, where the repository is private and a missing deploy
# key means no checkout at all.
#
# A deploy key is therefore about *pushing*, and it is optional.
# When one is present on the VM host the clone uses SSH and the
# agent can push; when it is not, the clone uses HTTPS and a push
# fails asking for credentials, which is the intended read-only
# state rather than a fault.
#
# Scoping matters if you do add one: a deploy key grants access to
# this single repository and nothing else on the account, and it
# is revoked in the repository's Settings > Deploy keys, which
# takes effect immediately and affects nothing else.
#
# The key is never stored in this repository. See the Vagrantfile
# for where it lives and why.
DEPLOY_KEY="$HOME/.ssh/kozmotic-deploy-key"
REPO_DIR="$HOME/kozmotic"

if [ -f "$DEPLOY_KEY" ]; then
  chmod 600 "$DEPLOY_KEY"

  # GitHub's host keys are taken from its published metadata over
  # HTTPS, not from whatever answers on port 22. `ssh-keyscan` and
  # `StrictHostKeyChecking=accept-new` both trust the first key
  # offered, which is the one thing host-key checking exists to
  # prevent. A dedicated known_hosts file keeps this independent
  # of anything else in ~/.ssh.
  mkdir -p "$HOME/.ssh"
  chmod 700 "$HOME/.ssh"
  curl -fsSL https://api.github.com/meta > "$WORK/gh-meta.json"
  jq -r '.ssh_keys[] | "github.com " + .' \
    "$WORK/gh-meta.json" > "$WORK/known_hosts_github"
  [ -s "$WORK/known_hosts_github" ] ||
    fail "no ssh host keys found in api.github.com/meta"
  install -m 0644 "$WORK/known_hosts_github" \
    "$HOME/.ssh/known_hosts_github"

  GIT_SSH_COMMAND="ssh -i $DEPLOY_KEY -o IdentitiesOnly=yes"
  GIT_SSH_COMMAND="$GIT_SSH_COMMAND -o StrictHostKeyChecking=yes"
  GIT_SSH_COMMAND="$GIT_SSH_COMMAND -o UserKnownHostsFile=$HOME/.ssh/known_hosts_github"
  export GIT_SSH_COMMAND

  CLONE_URL="git@github.com:breki/kozmotic.git"
  CLONE_KIND="ssh (deploy key present, pushing enabled)"
else
  GIT_SSH_COMMAND=""
  CLONE_URL="https://github.com/breki/kozmotic.git"
  CLONE_KIND="https (no deploy key, so this checkout is read-only)"
fi

# Cloned only when absent, and never pulled, reset or cleaned.
# This checkout is where the agent works, so it will hold
# uncommitted changes; a provisioning step that "brought it up to
# date" would silently destroy them. Updating the checkout is the
# agent's job, not this script's.
if [ ! -d "$REPO_DIR/.git" ]; then
  log "cloning kozmotic into $REPO_DIR over $CLONE_KIND"
  if ! git clone "$CLONE_URL" "$REPO_DIR"; then
    echo "" >&2
    echo "Cloning kozmotic failed." >&2
    if [ -n "$GIT_SSH_COMMAND" ]; then
      echo "A deploy key is present, so the most likely cause is" >&2
      echo "that it has not been added to the repository yet: add" >&2
      echo "the contents of" >&2
      echo "  ~/.secrets/kozmotic-deploy-key.pub  (on the VM host)" >&2
      echo "under Settings > Deploy keys, with write access." >&2
    else
      echo "The clone is over HTTPS and needs no credential, so" >&2
      echo "this is most likely a network problem on the VM." >&2
    fi
    exit 1
  fi
else
  log "kozmotic checkout already present, leaving it untouched"
fi

# Recorded in the checkout so the agent's own git commands use the
# key without needing the environment variable set.
#
# The remote is realigned as well, and that is not redundant. A VM
# first provisioned without a key has an https remote, and adding
# the key later does not change it: git would keep using https,
# ignore the key entirely, and prompt for a username and password
# on the first push -- from a VM where nobody is watching to type
# one. Setting both means "add a key, provision again" is enough
# to make pushing work, which is what that instruction implies.
#
# Only the URL is touched. The checkout itself is never pulled,
# reset or cleaned: it is where the agent works, so it will hold
# uncommitted changes, and a step that "brought it up to date"
# would silently destroy them.
if [ -n "$GIT_SSH_COMMAND" ]; then
  git -C "$REPO_DIR" config core.sshCommand "$GIT_SSH_COMMAND"

  CURRENT_REMOTE="$(git -C "$REPO_DIR" remote get-url origin 2>/dev/null || true)"
  SSH_REMOTE="git@github.com:breki/kozmotic.git"
  if [ "$CURRENT_REMOTE" != "$SSH_REMOTE" ]; then
    log "switching origin from $CURRENT_REMOTE to ssh"
    git -C "$REPO_DIR" remote set-url origin "$SSH_REMOTE"
  fi
fi

# An identity is required or every commit fails, and it comes from
# vagrant/local.env rather than from this file. Hardcoding one
# here would put one developer's name and address into a committed
# file, and every other developer's VM would then author commits
# as them.
#
# Set on the repository rather than globally, so it applies only
# where it is meant to.
if [ -n "$GIT_USER_NAME" ] && [ -n "$GIT_USER_EMAIL" ]; then
  git -C "$REPO_DIR" config user.name "$GIT_USER_NAME"
  git -C "$REPO_DIR" config user.email "$GIT_USER_EMAIL"
  log "git identity: $GIT_USER_NAME <$GIT_USER_EMAIL>"
else
  warn "no git identity configured, so commits in this VM will
   fail with 'Please tell me who you are'. Copy
   vagrant/local.env.sample to vagrant/local.env, set
   GIT_USER_NAME and GIT_USER_EMAIL, and provision again."
fi

log "kozmotic: $(git -C "$REPO_DIR" rev-parse --abbrev-ref HEAD) at $(git -C "$REPO_DIR" rev-parse --short HEAD)"

# --------------------------------------------------------------
# Land in the checkout
# --------------------------------------------------------------
# `bombyx shell` opens in the home directory, which is never where
# the work is.
#
# Vagrant has its own facility for this -- `config.ssh.extra_args`
# in the Vagrantfile, set to something like
# ["-t", "cd ~/kozmotic; exec bash --login"] -- and it is
# deliberately not used. Vagrant appends those arguments *before*
# the command given to `vagrant ssh -c`, and ssh joins trailing
# arguments into a single remote command, so setting it would
# corrupt every `vagrant ssh -c ...` run against this VM. That is
# the form every scripted check uses.
#
# Guarded three ways: interactive shells only, so `ssh vm cmd` and
# provisioning are untouched; only when the shell started in
# $HOME, so an explicit cd elsewhere is not overridden; and only
# when the directory exists, so a VM without a clone still opens a
# working shell.
CD_MARKER="# provisioned: open in the checkout"
if ! grep -qF "$CD_MARKER" "$HOME/.bashrc"; then
  log "setting the shell to open in $REPO_DIR"
  cat >> "$HOME/.bashrc" <<BASHRC

$CD_MARKER
case \$- in
  *i*) [ "\$PWD" = "\$HOME" ] && [ -d "$REPO_DIR" ] && cd "$REPO_DIR" ;;
esac
BASHRC
fi

log "provisioning complete"
