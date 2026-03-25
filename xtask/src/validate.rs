use crate::helpers::{elapsed_str, run_cargo, step_output};
use std::time::Instant;

pub fn validate() -> Result<(), String> {
    let total_start = Instant::now();
    let total_steps = 4;

    // Step 1: Format check
    let start = Instant::now();
    let output = run_cargo(&["fmt", "--all", "--", "--check"])?;
    if !output.status.success() {
        step_output(1, total_steps, "Format", "FAILED", "");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("{stderr}{stdout}");
        std::process::exit(2);
    }
    step_output(1, total_steps, "Format", "OK", &elapsed_str(start));

    // Step 2: Clippy
    let start = Instant::now();
    let output = run_cargo(&["clippy", "--all-targets", "--", "-D", "warnings"])?;
    if !output.status.success() {
        step_output(2, total_steps, "Clippy", "FAILED", "");
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{stderr}");
        std::process::exit(3);
    }
    step_output(2, total_steps, "Clippy", "OK", &elapsed_str(start));

    // Step 3: Tests
    let start = Instant::now();
    let output = run_cargo(&["test"])?;
    if !output.status.success() {
        step_output(3, total_steps, "Test", "FAILED", "");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{stderr}{stdout}");
        std::process::exit(4);
    }
    step_output(3, total_steps, "Test", "OK", &elapsed_str(start));

    // Step 4: Coverage (report only, no threshold)
    let start = Instant::now();
    let coverage = run_coverage();
    match coverage {
        Ok(pct) => {
            step_output(
                4,
                total_steps,
                "Coverage",
                "OK",
                &format!("{pct:.1}%, {}", elapsed_str(start)),
            );
        }
        Err(e) => {
            step_output(
                4,
                total_steps,
                "Coverage",
                "SKIP",
                &format!("{e}, {}", elapsed_str(start)),
            );
        }
    }

    println!("Validate OK ({})", elapsed_str(total_start));
    Ok(())
}

/// Run cargo llvm-cov and extract the total line coverage percentage.
/// Returns Ok(percentage) or Err with a reason string.
fn run_coverage() -> Result<f64, String> {
    let output = run_cargo(&["llvm-cov", "--package", "kozmotic", "--bins", "--tests"])?;

    if !output.status.success() {
        return Err("llvm-cov not available".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_coverage_total(&stdout)
}

/// Parse the TOTAL line from cargo llvm-cov text output.
/// Looks for "TOTAL" line and extracts the line coverage percentage.
fn parse_coverage_total(output: &str) -> Result<f64, String> {
    for line in output.lines() {
        if line.starts_with("TOTAL") {
            // Format: TOTAL  regions  missed  cover%  functions  missed  cover%  lines  missed  cover%
            // The line coverage % is the last percentage before any branch columns
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Find percentages (contain %)
            let percentages: Vec<f64> = parts
                .iter()
                .filter(|p| p.ends_with('%'))
                .filter_map(|p| p.trim_end_matches('%').parse::<f64>().ok())
                .collect();
            // Third percentage is line coverage
            if percentages.len() >= 3 {
                return Ok(percentages[2]);
            }
        }
    }
    Err("could not parse coverage".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coverage_total() {
        let output = "\
Filename                      Regions    Missed Regions     Cover   Functions  Missed Functions  Executed       Lines      Missed Lines     Cover    Branches   Missed Branches     Cover
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
agent_ping.rs                     396               230    41.92%          18                 9    50.00%         242               131    45.87%           0                 0         -
main.rs                            50                 4    92.00%           2                 1    50.00%          44                 3    93.18%           0                 0         -
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
TOTAL                             571               277    51.49%          27                12    55.56%         380               162    57.37%           0                 0         -";
        assert_eq!(parse_coverage_total(output).unwrap(), 57.37);
    }

    #[test]
    fn test_parse_coverage_no_total() {
        let output = "some random output";
        assert!(parse_coverage_total(output).is_err());
    }
}
