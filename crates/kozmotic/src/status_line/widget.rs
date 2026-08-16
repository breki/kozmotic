//! The set of widgets a `--show` spec may name.
//!
//! Widget selection used to be stringly typed end to end: `--show`
//! was a raw `String`, and four `render(name: &str, ..)` functions
//! each ended in `_ => None`. That made "unknown widget" and "this
//! widget has nothing to show right now" the same value, so a
//! misspelled `--show contxt` silently produced a shorter line with
//! no diagnostic anywhere — and the valid names lived in four match
//! statements, `--help`, and the README, which drifted apart.
//!
//! With one enum, `--show` is validated once at parse time, the
//! compiler enforces that every family handles every name it claims,
//! and `None` from a family means only "not mine".

/// A widget that can appear in the status line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    // Session payload.
    Model,
    Context,
    Cost,
    CostRate,
    Lines,
    Duration,
    ApiDuration,
    Tokens,
    Directory,
    Session,
    RateLimit,
    RateLimit7d,
    Vim,
    Worktree,
    Agent,
    // Git.
    GitBranch,
    GitAhead,
    GitFiles,
    GitLines,
    LastCommit,
    GitStatus,
    // Host.
    Host,
    Ram,
    Disk,
    // External.
    ApiStatus,
}

impl Widget {
    /// Every widget, in the order they are documented.
    pub const ALL: &'static [Widget] = &[
        Widget::Model,
        Widget::Context,
        Widget::Cost,
        Widget::CostRate,
        Widget::Lines,
        Widget::Duration,
        Widget::ApiDuration,
        Widget::Tokens,
        Widget::Directory,
        Widget::Session,
        Widget::RateLimit,
        Widget::RateLimit7d,
        Widget::Vim,
        Widget::Worktree,
        Widget::Agent,
        Widget::GitBranch,
        Widget::GitAhead,
        Widget::GitFiles,
        Widget::GitLines,
        Widget::LastCommit,
        Widget::GitStatus,
        Widget::Host,
        Widget::Ram,
        Widget::Disk,
        Widget::ApiStatus,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Widget::Model => "model",
            Widget::Context => "context",
            Widget::Cost => "cost",
            Widget::CostRate => "cost-rate",
            Widget::Lines => "lines",
            Widget::Duration => "duration",
            Widget::ApiDuration => "api-duration",
            Widget::Tokens => "tokens",
            Widget::Directory => "directory",
            Widget::Session => "session",
            Widget::RateLimit => "rate-limit",
            Widget::RateLimit7d => "rate-limit-7d",
            Widget::Vim => "vim",
            Widget::Worktree => "worktree",
            Widget::Agent => "agent",
            Widget::GitBranch => "git-branch",
            Widget::GitAhead => "git-ahead",
            Widget::GitFiles => "git-files",
            Widget::GitLines => "git-lines",
            Widget::LastCommit => "last-commit",
            Widget::GitStatus => "git-status",
            Widget::Host => "host",
            Widget::Ram => "ram",
            Widget::Disk => "disk",
            Widget::ApiStatus => "api-status",
        }
    }
}

impl std::fmt::Display for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown widget {name:?}; valid widgets: {}", valid())]
pub struct UnknownWidget {
    pub name: String,
}

fn valid() -> String {
    Widget::ALL
        .iter()
        .map(|w| w.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

impl std::str::FromStr for Widget {
    type Err = UnknownWidget;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Widget::ALL
            .iter()
            .copied()
            .find(|w| w.as_str() == s)
            .ok_or_else(|| UnknownWidget {
                name: s.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_round_trips_through_its_name() {
        for widget in Widget::ALL {
            let parsed: Widget = widget.as_str().parse().unwrap();
            assert_eq!(parsed, *widget, "{widget}");
        }
    }

    #[test]
    fn all_lists_every_variant_exactly_once() {
        let mut names: Vec<_> =
            Widget::ALL.iter().map(|w| w.as_str()).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate entry in Widget::ALL");
    }

    #[test]
    fn an_unknown_name_says_what_is_valid() {
        let err = "contxt".parse::<Widget>().unwrap_err();
        assert_eq!(err.name, "contxt");
        let msg = err.to_string();
        assert!(msg.contains("contxt"), "{msg}");
        assert!(msg.contains("context"), "{msg}");
    }
}
