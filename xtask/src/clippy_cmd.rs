use crate::helpers::run_cargo_capture;

const CLIPPY_ARGS: &[&str] = &[
    "clippy",
    "--workspace",
    "--all-targets",
    "--",
    "-D",
    "warnings",
];

const MAX_WARNING_LINES: usize = 10;

pub fn clippy() -> Result<(), String> {
    let r = clippy_check()?;
    match r.error {
        None => {
            println!("Clippy OK");
            Ok(())
        }
        Some(err) => {
            eprintln!("FAILED: clippy warning(s)\n");
            for line in r.items.iter().take(MAX_WARNING_LINES) {
                eprintln!("  {line}");
            }
            if r.items.len() > MAX_WARNING_LINES {
                eprintln!(
                    "  ... and {} more",
                    r.items.len() - MAX_WARNING_LINES
                );
            }
            Err(err)
        }
    }
}

pub struct ClippyResult {
    pub items: Vec<String>,
    pub error: Option<String>,
}

pub fn clippy_check() -> Result<ClippyResult, String> {
    let output = run_cargo_capture(CLIPPY_ARGS)?;

    if output.status.success() {
        return Ok(ClippyResult {
            items: vec![],
            error: None,
        });
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let items: Vec<String> = extract_warning_lines(&stderr)
        .into_iter()
        .map(String::from)
        .collect();

    Ok(ClippyResult {
        error: Some("clippy warning(s)".into()),
        items,
    })
}

fn extract_warning_lines(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter(|l| l.starts_with("warning:") || l.starts_with("error["))
        .filter(|l| {
            !l.contains("emitted")
                && !l.contains("build failed")
                && !l.contains("generated")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_STDERR: &str = "\
warning: used `sort` on primitive type `str`
    --> crates/kozmotic/src/lib.rs:10:9
warning: `kozmotic` (bin \"kozmotic\" test) \
generated 1 warning (1 duplicate)
error[E0425]: cannot find value `x`
    --> crates/kozmotic/src/lib.rs:10:5
error: could not compile `kozmotic`
warning: build failed, waiting for other jobs
warning: 2 warnings emitted";

    #[test]
    fn extracts_warnings_and_errors() {
        let lines = extract_warning_lines(SAMPLE_STDERR);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("sort"));
        assert!(lines[1].contains("E0425"));
    }

    #[test]
    fn empty_input() {
        let lines = extract_warning_lines("");
        assert!(lines.is_empty());
    }

    #[test]
    fn clean_output_gives_empty() {
        let stderr = "    Checking kozmotic v1.0.0\n\
            Finished `dev` profile";
        let lines = extract_warning_lines(stderr);
        assert!(lines.is_empty());
    }
}
