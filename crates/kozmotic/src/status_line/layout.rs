//! Turning rendered widgets into a line.
//!
//! A line spec may carry a `~`, which splits it into a left group
//! that flows from the start of the line and a right group pushed
//! against the terminal's right edge. Padding has to be measured in
//! *display columns*, which is neither the byte length nor the char
//! count: widgets are full of ANSI colour escapes that occupy no
//! columns at all, and a branch name may contain double-width
//! characters.

use unicode_width::UnicodeWidthChar;

use super::widget::{UnknownWidget, Widget};

/// Marker inside a line spec: widgets after it are right-aligned.
pub const RIGHT_MARKER: char = '~';

/// Fallback when no width is given and none can be detected.
const DEFAULT_WIDTH: usize = 80;

/// Upper bound on the width we will pad to.
///
/// `COLUMNS` is inherited from the environment, so a nonsense value
/// arrives without the user ever typing it, and the padding it
/// produces is allocated on *every* render: unclamped, a `usize`
/// reaches `" ".repeat(pad)` and either panics with a capacity
/// overflow or quietly emits megabytes into the host's status bar.
const MAX_WIDTH: usize = 1000;

/// A line of the status bar, split into its two alignment groups.
#[derive(Debug)]
pub struct LineSpec {
    pub left: Vec<Widget>,
    pub right: Vec<Widget>,
}

impl LineSpec {
    /// Parse one line of a `--show` value. Everything before the
    /// first `~` flows left; everything after is right-aligned.
    ///
    /// Fallible: an unrecognised name is reported rather than
    /// silently dropped, which is the whole point of parsing into
    /// [`Widget`] here instead of matching strings at render time.
    pub fn parse(spec: &str) -> Result<Self, UnknownWidget> {
        match spec.split_once(RIGHT_MARKER) {
            Some((left, right)) => Ok(Self {
                left: widgets(left)?,
                right: widgets(right)?,
            }),
            None => Ok(Self {
                left: widgets(spec)?,
                right: Vec::new(),
            }),
        }
    }
}

fn widgets(spec: &str) -> Result<Vec<Widget>, UnknownWidget> {
    spec.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect()
}

/// Visible width of a rendered widget, ignoring ANSI escape
/// sequences.
///
/// A CSI sequence is `ESC [`, then parameter and intermediate bytes,
/// then a final byte in `0x40..=0x7E`. The opening `[` is itself in
/// that range, so the scan for the final byte must start *after* it.
pub fn display_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            width += UnicodeWidthChar::width(c).unwrap_or(0);
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            for byte in chars.by_ref() {
                if ('\x40'..='\x7e').contains(&byte) {
                    break;
                }
            }
        } else {
            // Two-character escape, e.g. ESC c. A trailing lone ESC
            // simply ends the string.
            chars.next();
        }
    }
    width
}

/// Join the two groups into a finished line, padding between them so
/// the right group ends at `width`.
///
/// When the groups cannot both fit, they are separated by a single
/// space rather than truncated: cutting a string mid-escape would
/// leave the terminal coloured for the rest of the session, and a
/// slightly-too-long line merely wraps.
pub fn compose(
    left: &[String],
    right: &[String],
    separator: &str,
    width: usize,
) -> String {
    let left_text = left.join(separator);
    if right.is_empty() {
        return left_text;
    }
    let right_text = right.join(separator);
    // `compose` is public and takes the width it is handed, so the
    // allocation is bounded here too rather than trusting every
    // caller to have gone through `resolve_width`.
    if left_text.is_empty() {
        let pad = width
            .saturating_sub(display_width(&right_text))
            .min(MAX_WIDTH);
        return format!("{}{right_text}", " ".repeat(pad));
    }

    let used = display_width(&left_text) + display_width(&right_text);
    let pad = width.saturating_sub(used).clamp(1, MAX_WIDTH);
    format!("{left_text}{}{right_text}", " ".repeat(pad))
}

/// Resolve the width to align against: the explicit flag, else the
/// `COLUMNS` environment variable, else the real terminal, else a
/// conventional 80.
///
/// The probe matters because Claude Code pipes our stdout, so a
/// terminal query against the pipe reports nothing; `terminal_size`
/// falls back to the controlling terminal.
///
/// Values outside `1..=MAX_WIDTH` are treated as absent rather than
/// honoured — see [`MAX_WIDTH`].
pub fn resolve_width(explicit: Option<usize>) -> usize {
    let sane = |w: &usize| (1..=MAX_WIDTH).contains(w);
    explicit
        .filter(sane)
        .or_else(|| std::env::var("COLUMNS").ok()?.trim().parse::<usize>().ok())
        .filter(sane)
        .or_else(|| terminal_size::terminal_size().map(|(w, _)| w.0 as usize))
        .filter(sane)
        .unwrap_or(DEFAULT_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status_line::theme::{GREEN, RESET, label};

    #[test]
    fn display_width_ignores_ansi_escapes() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width(&format!("{GREEN}abc{RESET}")), 3);
        assert_eq!(display_width(&label("ctx")), 3);
    }

    #[test]
    fn display_width_counts_a_real_widget() {
        // What the `ctx` widget actually emits.
        let rendered = format!("{} {GREEN}42.5%{RESET}", label("ctx"));
        assert_eq!(display_width(&rendered), "ctx 42.5%".len());
    }

    #[test]
    fn display_width_handles_wide_characters() {
        // Arrows used by git-ahead are single-width.
        assert_eq!(display_width("↑2 ↓1"), 5);
        // CJK is double-width.
        assert_eq!(display_width("日本"), 4);
    }

    #[test]
    fn display_width_survives_a_truncated_escape() {
        assert_eq!(display_width("abc\x1b["), 3);
    }

    #[test]
    fn parse_without_marker_is_all_left() {
        let spec = LineSpec::parse("model, context ,cost").unwrap();
        assert_eq!(
            spec.left,
            vec![Widget::Model, Widget::Context, Widget::Cost]
        );
        assert!(spec.right.is_empty());
    }

    #[test]
    fn parse_splits_on_the_marker() {
        let spec = LineSpec::parse("model,context~cost,rate-limit").unwrap();
        assert_eq!(spec.left, vec![Widget::Model, Widget::Context]);
        assert_eq!(spec.right, vec![Widget::Cost, Widget::RateLimit]);
    }

    #[test]
    fn parse_allows_an_empty_left_group() {
        let spec = LineSpec::parse("~cost").unwrap();
        assert!(spec.left.is_empty());
        assert_eq!(spec.right, vec![Widget::Cost]);
    }

    #[test]
    fn parse_rejects_an_unknown_widget() {
        let err = LineSpec::parse("model,contxt").unwrap_err();
        assert_eq!(err.name, "contxt");
    }

    #[test]
    fn compose_without_right_group_is_a_plain_join() {
        let left = vec!["a".to_string(), "b".to_string()];
        assert_eq!(compose(&left, &[], " | ", 80), "a | b");
    }

    #[test]
    fn compose_pads_to_the_given_width() {
        let left = vec!["ab".to_string()];
        let right = vec!["cd".to_string()];
        let out = compose(&left, &right, " | ", 10);
        assert_eq!(out, "ab      cd");
        assert_eq!(display_width(&out), 10);
    }

    #[test]
    fn compose_measures_padding_in_visible_columns() {
        // The colour codes must not eat into the padding.
        let left = vec![format!("{GREEN}ab{RESET}")];
        let right = vec![format!("{GREEN}cd{RESET}")];
        let out = compose(&left, &right, " | ", 10);
        assert_eq!(display_width(&out), 10);
    }

    #[test]
    fn compose_falls_back_to_one_space_when_too_narrow() {
        let left = vec!["aaaaaa".to_string()];
        let right = vec!["bbbbbb".to_string()];
        // Overflowing is better than truncating mid-escape.
        assert_eq!(compose(&left, &right, " | ", 4), "aaaaaa bbbbbb");
    }

    #[test]
    fn compose_right_only_is_flushed_right() {
        let right = vec!["cd".to_string()];
        assert_eq!(compose(&[], &right, " | ", 6), "    cd");
    }

    #[test]
    fn resolve_width_prefers_the_explicit_value() {
        assert_eq!(resolve_width(Some(123)), 123);
    }

    #[test]
    fn resolve_width_ignores_a_zero_flag() {
        // 0 would make every line pad to nothing; treat it as unset.
        assert!(resolve_width(Some(0)) > 0);
    }

    #[test]
    fn resolve_width_always_returns_something_usable() {
        assert!(resolve_width(None) > 0);
    }
}
