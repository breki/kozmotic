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
//!
//! One widget is parameterised rather than fixed: `env:VAR` names a
//! variable the operator chooses, so it carries an [`EnvSpec`]
//! instead of being a bare variant. Everything else here — `ALL`,
//! `as_str`, the round-trip test — is about the fixed names.

use super::env_var::{EnvSpec, PREFIX as ENV_PREFIX};

/// A widget that can appear in the status line.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Process environment; parameterised — see [`EnvSpec`].
    Env(EnvSpec),
}

impl Widget {
    /// Every fixed widget, in the order they are documented.
    ///
    /// `Env` is absent by construction: there is no finite list of
    /// environment variables to enumerate.
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

    /// The `--show` name of a fixed widget, or `None` for `Env`.
    ///
    /// `Env` has no static name — its spelling depends on the
    /// variable it was given, so [`std::fmt::Display`] is what
    /// renders it in full. Returning `None` rather than a stand-in
    /// keeps the invariant that whatever this yields parses back into
    /// the same widget.
    pub fn as_str(&self) -> Option<&'static str> {
        Some(match self {
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
            Widget::Env(_) => return None,
        })
    }
}

impl std::fmt::Display for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Widget::Env(spec) => spec.fmt(f),
            // Every other variant has a name; `as_str` only declines
            // for `Env`, which is handled above.
            fixed => f.write_str(fixed.as_str().unwrap_or_default()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown widget {name:?}; valid widgets: {}", valid())]
pub struct UnknownWidget {
    pub name: String,
}

fn valid() -> String {
    let mut names: Vec<&str> =
        Widget::ALL.iter().filter_map(Widget::as_str).collect();
    // Listed last because it is a form rather than a name.
    names.push("env:VAR[:label]");
    names.join(", ")
}

impl std::str::FromStr for Widget {
    type Err = UnknownWidget;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let unknown = || UnknownWidget {
            name: s.to_string(),
        };
        // `env:` claims the whole prefix, so a malformed one is a
        // typo rather than a name some other family might own.
        if s.starts_with(ENV_PREFIX) {
            return EnvSpec::parse(s).map(Widget::Env).ok_or_else(unknown);
        }
        Widget::ALL
            .iter()
            .find(|w| w.as_str() == Some(s))
            .cloned()
            .ok_or_else(unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_round_trips_through_its_name() {
        for widget in Widget::ALL {
            let name = widget.as_str().expect("a fixed widget has a name");
            assert_eq!(name.parse::<Widget>().unwrap(), *widget, "{widget}");
            assert_eq!(widget.to_string(), name, "{widget}");
        }
    }

    #[test]
    fn all_lists_every_variant_exactly_once() {
        let mut names: Vec<_> =
            Widget::ALL.iter().filter_map(Widget::as_str).collect();
        assert_eq!(
            names.len(),
            Widget::ALL.len(),
            "a fixed widget has no name"
        );
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate entry in Widget::ALL");
    }

    #[test]
    fn an_env_name_parses_into_its_spec() {
        let widget: Widget = "env:VMHOST:vm".parse().unwrap();
        assert_eq!(widget.to_string(), "env:VMHOST:vm");
        assert!(matches!(widget, Widget::Env(_)));
        // It has no fixed name, and `Display` is what spells it out.
        assert_eq!(widget.as_str(), None);
    }

    #[test]
    fn a_malformed_env_name_is_rejected() {
        // Not "declined and tried elsewhere": nothing else owns the
        // prefix, so falling through would report a confusing error.
        let err = "env:".parse::<Widget>().unwrap_err();
        assert_eq!(err.name, "env:");
        assert!(err.to_string().contains("env:VAR"), "{err}");
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
