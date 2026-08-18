//! Git-backed widgets and the per-render cache behind them.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use super::format;
use super::theme::{CYAN, GREEN, RED, RESET, YELLOW, dim, label};
use super::widget::Widget;

#[derive(Debug, Default, PartialEq)]
struct GitFileCounts {
    staged: usize,
    modified: usize,
    new: usize,
    deleted: usize,
}

// Parsing and rendering are kept free of the `git` process so they
// can be exercised with fixtures. Driving them through a real
// repository instead would make coverage depend on whether the
// working tree happened to be dirty when the suite ran.

/// Count files by state from `git status --porcelain` output.
fn parse_file_counts(stdout: &str) -> GitFileCounts {
    let mut counts = GitFileCounts::default();
    for line in stdout.lines() {
        if line.len() < 2 {
            continue;
        }
        let index = line.as_bytes()[0];
        let worktree = line.as_bytes()[1];
        if index == b'?' {
            counts.new += 1;
            continue;
        }
        match index {
            b'A' | b'M' | b'R' => counts.staged += 1,
            b'D' => {
                counts.staged += 1;
                counts.deleted += 1;
            }
            _ => {}
        }
        match worktree {
            b'M' => counts.modified += 1,
            b'D' => counts.deleted += 1,
            _ => {}
        }
    }
    counts
}

/// Where the current branch stands relative to its upstream.
///
/// `Tracked(0, 0)` and `Unknown` both render nothing; `NoUpstream`
/// renders a dimmed marker. Keeping them apart is the point: a
/// branch that tracks nothing cannot be pushed by a bare
/// `git push`, and an empty widget would say the opposite.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SyncState {
    /// Commits ahead and behind the upstream branch.
    Tracked(usize, usize),
    /// The branch exists and has no upstream configured.
    NoUpstream,
    /// Anything else: no repository, no `git`, a detached or unborn
    /// HEAD, or an upstream that is configured but whose
    /// remote-tracking ref is missing locally. Silent, because the
    /// widget has nothing it can say truthfully.
    Unknown,
}

/// Parse `git rev-list --left-right --count HEAD...@{upstream}`.
fn parse_ahead_behind(stdout: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = stdout.trim().split('\t').collect();
    if parts.len() == 2 {
        Some((parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0)))
    } else {
        None
    }
}

/// Number of changed files in a `git diff --numstat` output.
fn count_numstat_files(stdout: &str) -> usize {
    stdout.lines().filter(|l| !l.is_empty()).count()
}

/// Sum added/deleted line counts across numstat outputs. Binary
/// files (rows whose counts are `-`) are skipped.
fn sum_numstat_lines(outputs: &[&str]) -> (usize, usize) {
    let mut added = 0usize;
    let mut deleted = 0usize;
    for stdout in outputs {
        for line in stdout.lines() {
            let mut cols = line.split('\t');
            let a = cols.next().unwrap_or("");
            let d = cols.next().unwrap_or("");
            if a == "-" || d == "-" {
                continue;
            }
            added += a.parse::<usize>().unwrap_or(0);
            deleted += d.parse::<usize>().unwrap_or(0);
        }
    }
    (added, deleted)
}

fn render_file_counts(counts: &GitFileCounts) -> String {
    let mut parts = Vec::new();
    if counts.staged > 0 {
        parts.push(format!("{GREEN}{}staged{RESET}", counts.staged));
    }
    if counts.modified > 0 {
        parts.push(format!("{YELLOW}{}mod{RESET}", counts.modified));
    }
    if counts.new > 0 {
        parts.push(format!("{CYAN}{}new{RESET}", counts.new));
    }
    if counts.deleted > 0 {
        parts.push(format!("{RED}{}del{RESET}", counts.deleted));
    }
    if parts.is_empty() {
        format!("{} (clean)", label("git"))
    } else {
        format!("{} {}", label("git"), parts.join(" "))
    }
}

fn render_ahead_behind(ahead: usize, behind: usize) -> Option<String> {
    if ahead == 0 && behind == 0 {
        return None;
    }
    let mut parts = Vec::new();
    if ahead > 0 {
        parts.push(format!("{GREEN}↑{ahead}{RESET}"));
    }
    if behind > 0 {
        parts.push(format!("{RED}↓{behind}{RESET}"));
    }
    Some(parts.join(" "))
}

/// Decide the sync state from what git reported.
///
/// `counts` is the parsed `rev-list` output, `upstream` the raw
/// stdout of `for-each-ref` over the branch's ref — `None` when that
/// probe was not run or failed.
///
/// Pure, so the three classifications are testable without putting
/// the repository the suite runs in into a particular state.
fn classify_sync(
    counts: Option<(usize, usize)>,
    upstream: Option<&str>,
) -> SyncState {
    if let Some((ahead, behind)) = counts {
        return SyncState::Tracked(ahead, behind);
    }
    // `for-each-ref` prints one line per matching ref: the upstream's
    // short name, or an empty line when the branch tracks nothing.
    // No line at all means the ref does not exist -- an unborn branch
    // -- which is not the same as tracking nothing.
    match upstream.and_then(|out| out.lines().next()) {
        Some(line) if line.trim().is_empty() => SyncState::NoUpstream,
        // A named upstream here means `rev-list` failed for some
        // other reason, most often a configured upstream whose
        // remote-tracking ref was pruned. That branch *can* be
        // pushed, so claiming "no upstream" would be wrong.
        _ => SyncState::Unknown,
    }
}

/// Word a sync state for the status line, or `None` when it has
/// nothing worth saying.
///
/// Split from the probing so the decision is a pure function: the
/// probing depends on the repository the suite happens to run in,
/// this does not.
fn ahead_text(sync: SyncState) -> Option<String> {
    match sync {
        SyncState::Tracked(ahead, behind) => render_ahead_behind(ahead, behind),
        // Dim, not red: a branch with no upstream is worth noticing,
        // not an error. `git push` with no arguments fails in this
        // state, so an empty widget would be read as "nothing to
        // push" exactly when there is something to push.
        SyncState::NoUpstream => Some(dim("(no upstream)")),
        SyncState::Unknown => None,
    }
}

fn render_diff_lines(added: usize, deleted: usize) -> Option<String> {
    if added == 0 && deleted == 0 {
        None
    } else {
        Some(format!("{GREEN}+{added}{RESET}/{RED}-{deleted}{RESET}"))
    }
}

fn render_status_counts(staged: usize, modified: usize) -> Option<String> {
    if staged == 0 && modified == 0 {
        return None;
    }
    let mut parts = Vec::new();
    if staged > 0 {
        parts.push(format!("{GREEN}+{staged}{RESET}"));
    }
    if modified > 0 {
        parts.push(format!("{YELLOW}~{modified}{RESET}"));
    }
    Some(parts.join(" "))
}

/// Lazily-cached git command results, shared across all git-* widgets
/// in a single status-line invocation. Each underlying `git` process
/// is spawned at most once per render.
#[derive(Default)]
pub struct GitContext {
    /// Directory every `git` process is run in. The session's
    /// working directory, not ours — see [`GitContext::new`].
    dir: Option<PathBuf>,
    branch: OnceCell<Option<String>>,
    sync: OnceCell<SyncState>,
    porcelain: OnceCell<Option<String>>,
    numstat_unstaged: OnceCell<Option<String>>,
    numstat_staged: OnceCell<Option<String>>,
    last_commit: OnceCell<Option<String>>,
}

fn run_git(dir: Option<&Path>, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

impl GitContext {
    /// Bind the git widgets to the session's working directory.
    ///
    /// Without this every `git` call inherits the status-line
    /// process's own cwd, which is whatever Claude Code happened to
    /// spawn us from — so `git-branch` could describe one repository
    /// while `disk` (which already uses the session directory)
    /// describes another, on the same rendered line. Silent wrong
    /// data in a status bar is worse than an absent widget.
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir: Some(dir),
            ..Self::default()
        }
    }

    fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }
    fn branch(&self) -> Option<&str> {
        self.branch
            .get_or_init(|| {
                run_git(self.dir(), &["branch", "--show-current"])
                    .map(|s| format::sanitize(s.trim()))
                    .filter(|s| !s.is_empty())
            })
            .as_deref()
    }

    fn sync(&self) -> SyncState {
        *self.sync.get_or_init(|| self.probe_sync())
    }

    /// Ask git how the branch stands against its upstream.
    ///
    /// Called once per render through [`GitContext::sync`]. A branch
    /// with an upstream costs one process; one without costs three
    /// (`rev-list`, then `branch --show-current` and `for-each-ref`
    /// to tell "tracks nothing" from "cannot tell").
    fn probe_sync(&self) -> SyncState {
        let counts = run_git(
            self.dir(),
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        )
        .as_deref()
        .and_then(parse_ahead_behind);
        if counts.is_some() {
            return classify_sync(counts, None);
        }
        // The configuration is probed, not the resolved ref: a
        // pruned `refs/remotes/...` makes `rev-parse @{upstream}`
        // fail on a branch that still has an upstream and still
        // pushes. Git's error text is not matched either -- the
        // commands that report a missing upstream word it
        // differently, and the wording is translated.
        let upstream = self.branch().and_then(|branch| {
            run_git(
                self.dir(),
                &[
                    "for-each-ref",
                    "--format=%(upstream:short)",
                    &format!("refs/heads/{branch}"),
                ],
            )
        });
        classify_sync(counts, upstream.as_deref())
    }

    fn porcelain(&self) -> Option<&str> {
        self.porcelain
            .get_or_init(|| run_git(self.dir(), &["status", "--porcelain"]))
            .as_deref()
    }

    fn numstat_unstaged(&self) -> Option<&str> {
        self.numstat_unstaged
            .get_or_init(|| run_git(self.dir(), &["diff", "--numstat"]))
            .as_deref()
    }

    fn numstat_staged(&self) -> Option<&str> {
        self.numstat_staged
            .get_or_init(|| {
                run_git(self.dir(), &["diff", "--cached", "--numstat"])
            })
            .as_deref()
    }

    /// Age of HEAD as a compact duration string with minute
    /// granularity ("12m", "2h 15m", "3d 4h"). Computed from the
    /// author timestamp; returns `None` when not in a repo.
    fn last_commit(&self) -> Option<&str> {
        self.last_commit
            .get_or_init(|| {
                let raw = run_git(self.dir(), &["log", "-1", "--format=%at"])?;
                let ts: i64 = raw.trim().parse().ok()?;
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()?
                    .as_secs() as i64;
                let age_secs = (now - ts).max(0) as u64;
                Some(format::age_compact(age_secs))
            })
            .as_deref()
    }

    fn file_counts(&self) -> Option<GitFileCounts> {
        Some(parse_file_counts(self.porcelain()?))
    }

    /// (`staged_files_changed`, `unstaged_files_changed`) — counted from
    /// numstat row counts, not porcelain, to match historical behavior.
    fn status_counts(&self) -> Option<(usize, usize)> {
        Some((
            count_numstat_files(self.numstat_staged()?),
            count_numstat_files(self.numstat_unstaged()?),
        ))
    }

    /// Sum added/deleted line counts across both staged and unstaged
    /// changes.
    fn diff_lines(&self) -> Option<(usize, usize)> {
        Some(sum_numstat_lines(&[
            self.numstat_unstaged()?,
            self.numstat_staged()?,
        ]))
    }
}

/// Render a git-backed widget, or `None` when the name belongs to
/// another family or there is nothing to report.
pub fn render(widget: &Widget, git: &GitContext) -> Option<String> {
    match widget {
        Widget::GitBranch => git.branch().map(|b| format!("{CYAN}{b}{RESET}")),
        Widget::GitAhead => ahead_text(git.sync()),
        Widget::GitFiles => Some(render_file_counts(&git.file_counts()?)),
        Widget::GitLines => {
            let (added, deleted) = git.diff_lines()?;
            render_diff_lines(added, deleted)
        }
        Widget::LastCommit => {
            git.last_commit().map(|s| format!("{} {s}", label("last")))
        }
        Widget::GitStatus => {
            let (staged, modified) = git.status_counts()?;
            render_status_counts(staged, modified)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Only the assertion on the dimmed marker needs the raw code.
    use crate::status_line::theme::DIM;

    /// Real `git status --porcelain` output: staged add, staged
    /// modify, unstaged modify, staged-and-unstaged, deletion,
    /// rename, and an untracked file.
    const PORCELAIN: &str = "\
A  added.rs
M  staged.rs
 M dirty.rs
MM both.rs
D  gone.rs
R  renamed.rs
?? untracked.rs
";

    #[test]
    fn parse_file_counts_classifies_each_state() {
        let counts = parse_file_counts(PORCELAIN);
        // A, M, MM, D, R are staged; D also counts as deleted.
        assert_eq!(
            counts,
            GitFileCounts {
                staged: 5,
                modified: 2,
                new: 1,
                deleted: 1,
            }
        );
    }

    #[test]
    fn parse_file_counts_of_clean_tree_is_all_zero() {
        assert_eq!(parse_file_counts(""), GitFileCounts::default());
    }

    #[test]
    fn parse_file_counts_skips_short_lines() {
        assert_eq!(parse_file_counts("x\n\n"), GitFileCounts::default());
    }

    #[test]
    fn parse_ahead_behind_reads_both_columns() {
        assert_eq!(parse_ahead_behind("2\t1\n"), Some((2, 1)));
        assert_eq!(parse_ahead_behind("0\t0\n"), Some((0, 0)));
    }

    #[test]
    fn parse_ahead_behind_rejects_unexpected_shape() {
        assert_eq!(parse_ahead_behind(""), None);
        assert_eq!(parse_ahead_behind("2"), None);
    }

    #[test]
    fn count_numstat_files_counts_rows() {
        assert_eq!(count_numstat_files("1\t2\ta.rs\n3\t4\tb.rs\n"), 2);
        assert_eq!(count_numstat_files(""), 0);
    }

    #[test]
    fn sum_numstat_lines_adds_across_outputs() {
        let unstaged = "10\t2\ta.rs\n5\t1\tb.rs\n";
        let staged = "3\t4\tc.rs\n";
        assert_eq!(sum_numstat_lines(&[unstaged, staged]), (18, 7));
    }

    #[test]
    fn sum_numstat_lines_skips_binary_files() {
        // Binary rows report "-" instead of counts.
        assert_eq!(sum_numstat_lines(&["-\t-\timage.png\n"]), (0, 0));
        assert_eq!(sum_numstat_lines(&["-\t-\tx.png\n4\t1\ty.rs\n"]), (4, 1));
    }

    #[test]
    fn render_file_counts_lists_every_present_state() {
        let out = render_file_counts(&parse_file_counts(PORCELAIN));
        assert!(out.contains("5staged"));
        assert!(out.contains("2mod"));
        assert!(out.contains("1new"));
        assert!(out.contains("1del"));
    }

    #[test]
    fn render_file_counts_says_clean_when_nothing_changed() {
        let out = render_file_counts(&GitFileCounts::default());
        assert!(out.contains("(clean)"));
    }

    #[test]
    fn ahead_text_is_silent_when_in_sync() {
        // One of the two silent states -- see
        // `ahead_text_is_silent_when_it_cannot_tell` for the other.
        assert_eq!(ahead_text(SyncState::Tracked(0, 0)), None);
    }

    #[test]
    fn ahead_text_shows_only_the_nonzero_sides() {
        let ahead = ahead_text(SyncState::Tracked(2, 0)).expect("renders");
        assert!(ahead.contains("↑2") && !ahead.contains("↓"), "{ahead}");
        let behind = ahead_text(SyncState::Tracked(0, 3)).expect("renders");
        assert!(behind.contains("↓3") && !behind.contains("↑"), "{behind}");
        let both = ahead_text(SyncState::Tracked(2, 1)).expect("renders");
        assert!(both.contains("↑2") && both.contains("↓1"), "{both}");
    }

    #[test]
    fn classify_prefers_the_counts_when_git_answered() {
        assert_eq!(classify_sync(Some((2, 1)), None), SyncState::Tracked(2, 1));
        assert_eq!(classify_sync(Some((0, 0)), None), SyncState::Tracked(0, 0));
    }

    #[test]
    fn classify_reads_a_blank_upstream_as_tracking_nothing() {
        // `for-each-ref` printed a line for the branch, and it was
        // empty: the branch exists and tracks nothing.
        assert_eq!(classify_sync(None, Some("\n")), SyncState::NoUpstream);
        assert_eq!(classify_sync(None, Some("")), SyncState::Unknown);
    }

    #[test]
    fn classify_stays_silent_when_an_upstream_is_configured() {
        // rev-list failed but the branch does have an upstream --
        // typically its remote-tracking ref was pruned. It still
        // pushes, so "(no upstream)" would be a lie.
        let out = classify_sync(None, Some("origin/main\n"));
        assert_eq!(out, SyncState::Unknown);
    }

    #[test]
    fn classify_stays_silent_without_a_probe() {
        // No repository, no git, or a branch with no ref of its own
        // (an unborn HEAD, where `for-each-ref` prints nothing).
        assert_eq!(classify_sync(None, None), SyncState::Unknown);
    }

    #[test]
    fn ahead_text_speaks_up_when_there_is_no_upstream() {
        // The state the change exists for: `git push` would fail
        // here, and an empty widget reads as "nothing to push".
        let out = ahead_text(SyncState::NoUpstream).expect("should render");
        assert!(out.contains("no upstream"), "{out}");
        // Dimmed: something to notice, not an error.
        assert!(out.starts_with(DIM) && out.ends_with(RESET), "{out:?}");
    }

    #[test]
    fn ahead_text_is_silent_when_it_cannot_tell() {
        // No repository, no git, or a detached HEAD — none of which
        // the operator needs a status-line widget to tell them.
        assert_eq!(ahead_text(SyncState::Unknown), None);
    }

    #[test]
    fn render_diff_lines_hidden_when_nothing_changed() {
        assert_eq!(render_diff_lines(0, 0), None);
        let out = render_diff_lines(42, 7).expect("should render");
        assert!(out.contains("+42"));
        assert!(out.contains("-7"));
    }

    #[test]
    fn render_status_counts_shows_only_nonzero_sides() {
        assert_eq!(render_status_counts(0, 0), None);
        let staged = render_status_counts(2, 0).expect("should render");
        assert!(staged.contains("+2") && !staged.contains('~'));
        let modified = render_status_counts(0, 1).expect("should render");
        assert!(modified.contains("~1") && !modified.contains('+'));
    }

    #[test]
    fn foreign_widget_name_is_declined() {
        let git = GitContext::default();
        // A widget owned by another family is declined.
        assert_eq!(render(&Widget::Model, &git), None);
    }

    /// The widget names route to the right renderer. Values depend
    /// on the repository, so assert only that repeated calls agree —
    /// which also exercises the per-render cache.
    #[test]
    fn git_widgets_answer_consistently() {
        let git = GitContext::default();
        for widget in [
            Widget::GitBranch,
            Widget::GitAhead,
            Widget::GitFiles,
            Widget::GitLines,
            Widget::LastCommit,
            Widget::GitStatus,
        ] {
            let first = render(&widget, &git);
            assert_eq!(first, render(&widget, &git), "{widget} is not stable");
        }
    }
}
