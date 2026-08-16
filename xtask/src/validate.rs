use std::time::Instant;

use crate::clippy_cmd;
use crate::coverage;
use crate::fmt_cmd;
use crate::helpers::{elapsed_str, step_output};
use crate::test_cmd;

const TOTAL_STEPS: usize = 4;

pub fn validate() -> Result<(), String> {
    let overall_start = Instant::now();

    run_step(1, "Fmt", run_fmt)?;
    run_step(2, "Clippy", run_clippy)?;
    run_step(3, "Test", run_test)?;
    run_step(4, "Coverage", run_coverage)?;

    println!("Validate OK ({})", elapsed_str(overall_start));
    Ok(())
}

fn run_step(
    step: usize,
    name: &str,
    f: fn() -> Result<String, String>,
) -> Result<(), String> {
    let start = Instant::now();
    match f() {
        Ok(detail) => {
            let time = elapsed_str(start);
            let full = if detail.is_empty() {
                time
            } else {
                format!("{detail}, {time}")
            };
            step_output(step, TOTAL_STEPS, name, "OK", &full);
            Ok(())
        }
        Err(e) => {
            step_output(step, TOTAL_STEPS, name, "FAILED", "");
            Err(e)
        }
    }
}

fn run_fmt() -> Result<String, String> {
    fmt_cmd::fmt_check()?;
    Ok(String::new())
}

fn run_clippy() -> Result<String, String> {
    let r = clippy_cmd::clippy_check()?;
    match r.error {
        None => Ok(String::new()),
        Some(err) => {
            for line in r.items.iter().take(5) {
                eprintln!("  {line}");
            }
            Err(err)
        }
    }
}

fn run_test() -> Result<String, String> {
    test_cmd::test_check_xtask()?;
    Ok(String::new())
}

fn run_coverage() -> Result<String, String> {
    let r = coverage::coverage_check()?;
    match r.error {
        None => Ok(format!(
            "{:.1}% >= {}%",
            r.line_pct,
            coverage::OVERALL_THRESHOLD,
        )),
        Some(failure) => Err(coverage::format_failure(&failure)),
    }
}
