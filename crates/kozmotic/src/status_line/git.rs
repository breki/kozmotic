//! Git-backed widgets and the per-render cache behind them.

use std::cell::OnceCell;
use std::process::Command;
use std::time::SystemTime;

use super::format;
use super::theme::{CYAN, GREEN, RED, RESET, YELLOW, label};

struct GitFileCounts {
    staged: usize,
    modified: usize,
    new: usize,
    deleted: usize,
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
            let parts: Vec<&str> = stdout.trim().split('\t').collect();
            if parts.len() == 2 {
                Some((
                    parts[0].parse().unwrap_or(0),
                    parts[1].parse().unwrap_or(0),
                ))
            } else {
                None
            }
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
        let stdout = self.porcelain()?;
        let mut staged = 0;
        let mut modified = 0;
        let mut new = 0;
        let mut deleted = 0;
        for line in stdout.lines() {
            if line.len() < 2 {
                continue;
            }
            let index = line.as_bytes()[0];
            let worktree = line.as_bytes()[1];
            if index == b'?' {
                new += 1;
                continue;
            }
            match index {
                b'A' | b'M' | b'R' => staged += 1,
                b'D' => {
                    staged += 1;
                    deleted += 1;
                }
                _ => {}
            }
            match worktree {
                b'M' => modified += 1,
                b'D' => deleted += 1,
                _ => {}
            }
        }
        Some(GitFileCounts {
            staged,
            modified,
            new,
            deleted,
        })
    }

    /// (`staged_files_changed`, `unstaged_files_changed`) — counted from
    /// numstat row counts, not porcelain, to match historical behavior.
    fn status_counts(&self) -> Option<(usize, usize)> {
        let staged = self.numstat_staged()?;
        let modified = self.numstat_unstaged()?;
        let staged_count = staged.lines().filter(|l| !l.is_empty()).count();
        let modified_count = modified.lines().filter(|l| !l.is_empty()).count();
        Some((staged_count, modified_count))
    }

    /// Sum added/deleted line counts across both staged and unstaged
    /// changes. Binary files (numstat rows starting with `-`) are
    /// skipped.
    fn diff_lines(&self) -> Option<(usize, usize)> {
        let mut added = 0usize;
        let mut deleted = 0usize;
        for stdout in [self.numstat_unstaged()?, self.numstat_staged()?] {
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
        Some((added, deleted))
    }
}

/// Render a git-backed widget, or `None` when the name belongs to
/// another family or there is nothing to report.
pub fn render(name: &str, git: &GitContext) -> Option<String> {
    match name {
        "git-branch" => git.branch().map(|b| format!("{CYAN}{b}{RESET}")),
        "git-ahead" => {
            let (ahead, behind) = git.ahead_behind()?;
            if ahead == 0 && behind == 0 {
                None
            } else {
                let mut parts = Vec::new();
                if ahead > 0 {
                    parts.push(format!("{GREEN}↑{ahead}{RESET}"));
                }
                if behind > 0 {
                    parts.push(format!("{RED}↓{behind}{RESET}"));
                }
                Some(parts.join(" "))
            }
        }
        "git-files" => {
            let counts = git.file_counts()?;
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
                Some(format!("{} (clean)", label("git")))
            } else {
                Some(format!("{} {}", label("git"), parts.join(" ")))
            }
        }
        "git-lines" => {
            let (added, deleted) = git.diff_lines()?;
            if added == 0 && deleted == 0 {
                None
            } else {
                Some(format!("{GREEN}+{added}{RESET}/{RED}-{deleted}{RESET}"))
            }
        }
        "last-commit" => {
            git.last_commit().map(|s| format!("{} {s}", label("last")))
        }
        "git-status" => {
            let (staged, modified) = git.status_counts()?;
            if staged == 0 && modified == 0 {
                None
            } else {
                let mut parts = Vec::new();
                if staged > 0 {
                    parts.push(format!("{GREEN}+{staged}{RESET}"));
                }
                if modified > 0 {
                    parts.push(format!("{YELLOW}~{modified}{RESET}"));
                }
                Some(parts.join(" "))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_widget_name_is_declined() {
        let git = GitContext::default();
        assert_eq!(render("model", &git), None);
    }

    /// Results depend on the repository state, so assert only that
    /// each git widget answers without panicking and that the cache
    /// serves repeated calls.
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
