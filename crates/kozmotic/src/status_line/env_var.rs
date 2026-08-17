//! A widget that renders the value of an environment variable.
//!
//! Every other widget is a fixed name. Here the operator names the
//! variable in `--show`, so kozmotic can show a value it has no
//! knowledge of — which machine a VM runs on, a deployment target, a
//! cluster name.
//!
//! That also makes it the only widget whose text comes straight from
//! the environment, so both the value and the label go through
//! [`format::sanitize`] before they reach the terminal.

use super::format;
use super::theme::label;
use super::widget::Widget;

/// Prefix that marks a `--show` name as an environment-variable
/// widget.
pub const PREFIX: &str = "env:";

/// Which variable to read, and what to call it on the line.
///
/// Spelled `env:VAR`, or `env:VAR:label` when the bare value would
/// not say what it is. The label may contain `:`, but not `,`, `;`
/// or `~`: the `--show` grammar splits on those first, so they never
/// reach this parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvSpec {
    key: String,
    label: Option<String>,
}

impl EnvSpec {
    /// Parse a `--show` name into a spec, or `None` when it is not an
    /// `env:` name or names no variable at all (`env:`, `env::vm`).
    ///
    /// Surrounding whitespace is trimmed from both the name and the
    /// label, so `env:  PATH` and `env:PATH` are the same widget.
    pub fn parse(name: &str) -> Option<Self> {
        let rest = name.strip_prefix(PREFIX)?;
        // First colon only, so a label may itself contain one.
        let (key, given) = match rest.split_once(':') {
            Some((key, given)) => (key, Some(given)),
            None => (rest, None),
        };
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        Some(Self {
            key: key.to_owned(),
            label: given
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned),
        })
    }
}

impl std::fmt::Display for EnvSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{PREFIX}{}", self.key)?;
        match &self.label {
            Some(l) => write!(f, ":{l}"),
            None => Ok(()),
        }
    }
}

/// Render an env-backed widget, or `None` when the widget belongs to
/// another family, the variable is unset, or its value is blank.
pub fn render(widget: &Widget) -> Option<String> {
    match widget {
        // `var_os`, not `var`: a value that is not valid UTF-8 is
        // still a value, and `var` reports it as absent — which the
        // docs tell the operator means the variable is unset.
        Widget::Env(spec) => render_with(spec, |key| {
            std::env::var_os(key).map(|v| v.to_string_lossy().into_owned())
        }),
        _ => None,
    }
}

/// [`render`] against an arbitrary environment.
///
/// The lookup is a parameter so the tests never mutate the process
/// environment, which is global and would make them race each other
/// under the test harness's threads.
fn render_with<F>(spec: &EnvSpec, lookup: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    // The value is arbitrary text from the operator's environment. An
    // ESC in it would leave the terminal coloured for the rest of the
    // session, a newline would split the status bar across two lines
    // while `layout::display_width` still scored it as one, and a
    // 5000-character value would be re-allocated on every render.
    // `sanitize` strips control characters and caps the length.
    let value = format::sanitize(&lookup(&spec.key)?);
    let value = value.trim();
    // An unset variable, one set to "" or "   ", and one holding
    // nothing but control characters are the same thing to a reader,
    // and the line is better off without any of them. Widgets that
    // render nothing are dropped by the caller, so returning None is
    // how a widget opts out.
    if value.is_empty() {
        return None;
    }
    Some(match &spec.label {
        // The label comes from `--show`, which is on-disk config in a
        // project's `.claude/settings.json`, so it is no more trusted
        // than the value.
        Some(l) => format!("{} {value}", label(&format::sanitize(l))),
        None => value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status_line::theme::{DIM, RESET};

    /// A lookup with exactly one variable set.
    fn one(
        key: &'static str,
        value: &'static str,
    ) -> impl Fn(&str) -> Option<String> {
        move |k| (k == key).then(|| value.to_owned())
    }

    fn spec(name: &str) -> EnvSpec {
        EnvSpec::parse(name).unwrap_or_else(|| panic!("{name} should parse"))
    }

    #[test]
    fn renders_the_value_bare_by_default() {
        let out = render_with(&spec("env:VMHOST"), one("VMHOST", "bombyx"));
        assert_eq!(out, Some("bombyx".to_owned()));
    }

    #[test]
    fn renders_a_label_when_one_is_given() {
        let out = render_with(&spec("env:VMHOST:vm"), one("VMHOST", "bombyx"));
        assert_eq!(out, Some(format!("{DIM}vm{RESET} bombyx")));
    }

    #[test]
    fn an_unset_variable_renders_nothing() {
        assert_eq!(render_with(&spec("env:NOPE"), |_| None), None);
    }

    #[test]
    fn a_blank_value_renders_nothing() {
        // Whitespace too: a variable exported as "" by a wrapper
        // script is a variable with no value, and a widget that
        // renders a lone space is worse than one that is absent.
        for blank in ["", "   ", "\t"] {
            let out = render_with(&spec("env:V"), one("V", blank));
            assert_eq!(out, None, "{blank:?}");
        }
    }

    #[test]
    fn a_missing_variable_name_does_not_parse() {
        // `env:` and `env::vm` name nothing at all, so they are a
        // spelling mistake rather than a widget with nothing to show.
        assert_eq!(EnvSpec::parse("env:"), None);
        assert_eq!(EnvSpec::parse("env::vm"), None);
        assert_eq!(EnvSpec::parse("env:   :vm"), None);
    }

    #[test]
    fn a_foreign_widget_name_is_declined() {
        // The family contract: decline anything without the prefix,
        // so the dispatch chain can keep walking.
        for name in ["host", "environment", "envy", "model"] {
            assert_eq!(EnvSpec::parse(name), None, "{name}");
        }
        assert_eq!(render(&Widget::Model), None);
    }

    #[test]
    fn a_blank_label_is_the_same_as_no_label() {
        let out = render_with(&spec("env:V:  "), one("V", "x"));
        assert_eq!(out, Some("x".to_owned()));
    }

    #[test]
    fn a_label_may_contain_a_colon() {
        let out = render_with(&spec("env:V:a:b"), one("V", "x"));
        assert!(out.expect("renders").contains("a:b"));
    }

    #[test]
    fn control_characters_never_reach_the_terminal() {
        // An ESC would recolour the bar for the rest of the session
        // and a newline would split it in two, on every render.
        let out = render_with(&spec("env:V"), one("V", "a\x1b[31mb\nc"));
        assert_eq!(out, Some("a[31mbc".to_owned()));
    }

    #[test]
    fn a_value_of_only_control_characters_renders_nothing() {
        let out = render_with(&spec("env:V"), one("V", "\x1b\x07"));
        assert_eq!(out, None);
    }

    #[test]
    fn a_long_value_is_capped() {
        let long = "x".repeat(500);
        let out = render_with(&spec("env:V"), move |_| Some(long.clone()))
            .expect("renders");
        assert!(out.chars().count() <= 120, "{} chars", out.chars().count());
    }

    #[test]
    fn a_label_is_sanitized_too() {
        // `--show` is on-disk config, so a label is no more trusted
        // than the value it introduces.
        let out = render_with(&spec("env:V:la\x1b[31mbel"), one("V", "x"));
        // The theme's own DIM/RESET remain; the label's escape does
        // not, so it cannot outlive the widget it introduces.
        assert_eq!(out, Some(format!("{DIM}la[31mbel{RESET} x")));
    }

    #[test]
    fn whitespace_around_the_name_is_trimmed() {
        assert_eq!(spec("env:  PATH  ").to_string(), "env:PATH");
    }

    #[test]
    fn a_spec_displays_as_the_name_it_was_parsed_from() {
        // What an error message about this widget has to say.
        assert_eq!(spec("env:V").to_string(), "env:V");
        assert_eq!(spec("env:V:vm").to_string(), "env:V:vm");
    }

    #[test]
    fn reads_the_real_environment() {
        // The one test that goes through `render`, so the wiring
        // between it and `render_with` is covered rather than
        // assumed. PATH is set on every platform kozmotic targets.
        assert!(render(&Widget::Env(spec("env:PATH"))).is_some());
        let unset = Widget::Env(spec("env:KOZMOTIC_DEFINITELY_UNSET_XYZ"));
        assert_eq!(render(&unset), None);
    }
}
