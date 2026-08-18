//! ANSI palette and the shared visual vocabulary of the status
//! line. Keeping it in one place is what makes a glance mean the
//! same thing across widgets.

pub const DIM: &str = "\x1b[2m";
pub const RESET: &str = "\x1b[0m";
pub const CYAN: &str = "\x1b[36m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";

/// A widget's dimmed prefix, e.g. the `ctx` in `ctx 42.5%`.
pub fn label(name: &str) -> String {
    dim(name)
}

/// Dimmed text: a value the reader should notice without it
/// competing with the numbers, e.g. `git-ahead`'s `(no upstream)`.
pub fn dim(text: &str) -> String {
    format!("{DIM}{text}{RESET}")
}

/// Shared "how full is it" scale: green below half, yellow from half,
/// red from 80%. Used by `context`, `ram`, and `disk` so a glance
/// means the same thing everywhere on the line.
pub fn usage_color(pct: f64) -> &'static str {
    if pct >= 80.0 {
        RED
    } else if pct >= 50.0 {
        YELLOW
    } else {
        GREEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_color_thresholds() {
        assert_eq!(usage_color(0.0), GREEN);
        assert_eq!(usage_color(49.9), GREEN);
        assert_eq!(usage_color(50.0), YELLOW);
        assert_eq!(usage_color(79.9), YELLOW);
        assert_eq!(usage_color(80.0), RED);
        assert_eq!(usage_color(100.0), RED);
    }

    #[test]
    fn label_is_dimmed_and_reset() {
        assert_eq!(label("ctx"), format!("{DIM}ctx{RESET}"));
        assert_eq!(dim("(no upstream)"), format!("{DIM}(no upstream){RESET}"));
    }
}
