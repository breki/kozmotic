use crate::helpers::{elapsed_str, run_cargo, step_output};
use std::time::Instant;

pub fn validate() -> Result<(), String> {
    let total_start = Instant::now();
    let total_steps = 3;

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

    println!("Validate OK ({})", elapsed_str(total_start));
    Ok(())
}
