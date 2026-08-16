//! Git-backed widgets and the per-render cache behind them.

use std::cell::OnceCell;
use std::process::Command;
use std::time::SystemTime;

use super::format;
use super::theme::{CYAN, GREEN, RED, RESET, YELLOW, label};

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
    branch: OnceCell<Option<String>>,
    ahead_behind: OnceCell<Option<(usize, usize)>>,
    porcelain: OnceCell<Option<String>>,
    numstat_unstaged: OnceCell<Option<String>>,
    numstat_staged: OnceCell<Option<String>>,
    last_commit: OnceCell<Option<String>>,
}

fn run_git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

impl GitContext {
    fn branch(&self) -> Option<&str> {
        self.branch
            .get_or_init(|| {
                run_git(&["branch", "--show-current"])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .as_deref()
    }

    fn ahead_behind(&self) -> Option<(usize, usize)> {
        *self.ahead_behind.get_or_init(|| {
            let stdout = run_git(&[
                "rev-list",
                "--left-right",
                "--count",
                "HEAD...@{upstream}",
            ])?;
            parse_ahead_behind(&stdout)
        })
    }

    fn porcelain(&self) -> Option<&str> {
        self.porcelain
            .get_or_init(|| run_git(&["status", "--porcelain"]))
            .as_deref()
    }

    fn numstat_unstaged(&self) -> Option<&str> {
        self.numstat_unstaged
            .get_or_init(|| run_git(&["diff", "--numstat"]))
            .as_deref()
    }

    fn numstat_staged(&self) -> Option<&str> {
        self.numstat_staged
            .get_or_init(|| run_git(&["diff", "--cached", "--numstat"]))
            .as_deref()
    }

    /// Age of HEAD as a compact duration string with minute
    /// granularity ("12m", "2h 15m", "3d 4h"). Computed from the
    /// author timestamp; returns `None` when not in a repo.
    fn last_commit(&self) -> Option<&str> {
        self.last_commit
            .get_or_init(|| {
                let raw = run_git(&["log", "-1", "--format=%at"])?;
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
pub fn render(name: &str, git: &GitContext) -> Option<String> {
    match name {
        "git-branch" => git.branch().map(|b| format!("{CYAN}{b}{RESET}")),
        "git-ahead" => {
            let (ahead, behind) = git.ahead_behind()?;
            render_ahead_behind(ahead, behind)
        }
        "git-files" => Some(render_file_counts(&git.file_counts()?)),
        "git-lines" => {
            let (added, deleted) = git.diff_lines()?;
            render_diff_lines(added, deleted)
        }
        "last-commit" => {
            git.last_commit().map(|s| format!("{} {s}", label("last")))
        }
        "git-status" => {
            let (staged, modified) = git.status_counts()?;
            render_status_counts(staged, modified)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn render_ahead_behind_shows_only_nonzero_sides() {
        assert_eq!(render_ahead_behind(0, 0), None);
        let ahead = render_ahead_behind(2, 0).expect("should render");
        assert!(ahead.contains("↑2") && !ahead.contains("↓"));
        let behind = render_ahead_behind(0, 3).expect("should render");
        assert!(behind.contains("↓3") && !behind.contains("↑"));
        let both = render_ahead_behind(2, 3).expect("should render");
        assert!(both.contains("↑2") && both.contains("↓3"));
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
        assert_eq!(render("model", &git), None);
    }

    /// The widget names route to the right renderer. Values depend
    /// on the repository, so assert only that repeated calls agree —
    /// which also exercises the per-render cache.
    #[test]
    fn git_widgets_answer_consistently() {
        let git = GitContext::default();
        for widget in [
            "git-branch",
            "git-ahead",
            "git-files",
            "git-lines",
            "last-commit",
            "git-status",
        ] {
            let first = render(widget, &git);
            assert_eq!(first, render(widget, &git), "{widget} is not stable");
        }
    }
}
