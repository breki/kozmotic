use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::Instant;

/// Return the workspace root directory.
pub fn project_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate must be inside the workspace")
        .to_path_buf()
}

/// Run a cargo command with the given args, capturing stdout+stderr.
pub fn run_cargo(args: &[&str]) -> Result<Output, String> {
    Command::new("cargo")
        .current_dir(project_root())
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run cargo {}: {e}", args.first().unwrap_or(&"")))
}

/// Format a step output line for validation progress.
/// Example: `[1/3] Clippy...... OK (1.2s)`
pub fn step_output(step: usize, total: usize, name: &str, status: &str, detail: &str) {
    let padding = ".".repeat(12usize.saturating_sub(name.len()));
    if detail.is_empty() {
        println!("[{step}/{total}] {name}{padding} {status}");
    } else {
        println!("[{step}/{total}] {name}{padding} {status} ({detail})");
    }
}

/// Format elapsed seconds from an Instant.
pub fn elapsed_str(start: Instant) -> String {
    let secs = start.elapsed().as_secs_f64();
    format!("{secs:.1}s")
}
