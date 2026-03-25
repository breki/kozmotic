use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::path::PathBuf;

#[test]
fn test_help() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("CLI toolkit for AI agents"));
}

#[test]
fn test_example_json_output() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("example")
        .arg("--name")
        .arg("Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""))
        .stdout(predicate::str::contains("Hello, Test!"));
}

#[test]
fn test_example_human_output() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("--format")
        .arg("human")
        .arg("example")
        .arg("--name")
        .arg("Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, Test!"))
        .stdout(predicate::str::contains("status").not());
}

#[test]
fn test_version() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("kozmotic"));
}

// --- agent-ping tests ---

#[test]
fn test_agent_ping_list() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""))
        .stdout(predicate::str::contains("Stop"))
        .stdout(predicate::str::contains("StopFailure"))
        .stdout(predicate::str::contains("Notification"));
}

#[test]
fn test_agent_ping_list_human() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("--format")
        .arg("human")
        .arg("agent-ping")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Stop"))
        .stdout(predicate::str::contains("Notification"))
        .stdout(predicate::str::contains("status").not());
}

#[test]
fn test_agent_ping_dry_run() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--sound")
        .arg("Stop")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"played\": false"))
        .stdout(predicate::str::contains("Stop"));
}

#[test]
fn test_agent_ping_dry_run_freq() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--frequency")
        .arg("440")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"played\": false"))
        .stdout(predicate::str::contains("440"));
}

#[test]
fn test_agent_ping_dry_run_human() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("--format")
        .arg("human")
        .arg("agent-ping")
        .arg("--sound")
        .arg("Stop")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run] Would play: Stop"));
}

#[test]
fn test_agent_ping_unknown_preset() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--sound")
        .arg("UnknownPreset")
        .assert()
        .failure()
        .stderr(predicate::str::contains("UNKNOWN_PRESET"));
}

#[test]
fn test_agent_ping_missing_source() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .assert()
        .failure()
        .stderr(predicate::str::contains("MISSING_SOUND_SOURCE"));
}

#[test]
fn test_agent_ping_freq_low() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--frequency")
        .arg("10")
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_FREQUENCY"));
}

#[test]
fn test_agent_ping_freq_high() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--frequency")
        .arg("25000")
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_FREQUENCY"));
}

#[test]
fn test_agent_ping_file_not_found() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--file")
        .arg("nonexistent/path/sound.wav")
        .assert()
        .failure()
        .stderr(predicate::str::contains("FILE_NOT_FOUND"));
}

#[test]
fn test_agent_ping_volume_range() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--sound")
        .arg("Stop")
        .arg("--volume")
        .arg("1.5")
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_VOLUME"));
}

#[test]
fn test_agent_ping_case_insensitive() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("agent-ping")
        .arg("--sound")
        .arg("stop")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"played\": false"));
}

// --- status-line tests ---

const SAMPLE_STATUS_JSON: &str = r#"{
    "model": { "id": "claude-opus-4-6", "display_name": "Opus 4.6" },
    "context_window": { "used_percentage": 42.5, "remaining_percentage": 57.5 },
    "cost": { "total_cost_usd": 1.23, "total_lines_added": 150, "total_lines_removed": 30 }
}"#;

#[test]
fn test_status_line_default() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .write_stdin(SAMPLE_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("Opus 4.6"))
        .stdout(predicate::str::contains("42.5%"))
        .stdout(predicate::str::contains("\x1b[32m")) // green for <50%
        .stdout(predicate::str::contains("$1.23"));
}

#[test]
fn test_status_line_show_flag() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("model")
        .write_stdin(SAMPLE_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("Opus 4.6"))
        .stdout(predicate::str::contains("$").not());
}

#[test]
fn test_status_line_custom_separator() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--separator")
        .arg(" :: ")
        .write_stdin(SAMPLE_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains(" :: "));
}

#[test]
fn test_status_line_context_red() {
    let json = r#"{
        "model": { "id": "x", "display_name": "X" },
        "context_window": { "used_percentage": 85.0 },
        "cost": {}
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("context")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[31m")); // red
}

#[test]
fn test_status_line_rate_limit() {
    let json = r#"{
        "model": {},
        "context_window": {},
        "cost": {},
        "rate_limits": {
            "five_hour": { "used_percentage": 73.2 }
        }
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("rate-limit")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("73%"));
}

#[test]
fn test_status_line_vim_mode() {
    let json = r#"{
        "model": {},
        "context_window": {},
        "cost": {},
        "vim": { "mode": "NORMAL" }
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("vim")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("NORMAL"));
}

const FULL_STATUS_JSON: &str = r#"{
    "model": { "id": "claude-opus-4-6", "display_name": "Opus 4.6" },
    "context_window": {
        "used_percentage": 42.5,
        "total_input_tokens": 15234,
        "total_output_tokens": 4521
    },
    "cost": {
        "total_cost_usd": 1.23,
        "total_duration_ms": 754000,
        "total_api_duration_ms": 130000,
        "total_lines_added": 150,
        "total_lines_removed": 30
    },
    "workspace": { "current_dir": "/home/user/projects/kozmotic" },
    "session_id": "abc123def456",
    "agent": { "name": "security-reviewer" },
    "worktree": { "name": "my-feature" }
}"#;

#[test]
fn test_status_line_duration() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("duration")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("12m 34s"));
}

#[test]
fn test_status_line_api_duration() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("api-duration")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("2m 10s"));
}

#[test]
fn test_status_line_tokens() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("tokens")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("15.2k in / 4.5k out"));
}

#[test]
fn test_status_line_directory() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("directory")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("kozmotic"));
}

#[test]
fn test_status_line_session() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("session")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("abc123de"));
}

#[test]
fn test_status_line_agent() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("agent")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("security-reviewer"));
}

#[test]
fn test_status_line_worktree() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("worktree")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("my-feature"));
}

#[test]
fn test_status_line_git_branch() {
    // We're in a git repo, so this should return a branch name
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("git-branch")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("main"));
}

#[test]
fn test_status_line_git_files() {
    // Create a temp file to guarantee at least one modified file
    let tmp = std::env::temp_dir().join("kozmotic-git-files-test");
    let _ = std::fs::write(&tmp, "test");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    // Just check it runs successfully - exact counts depend on repo state
    cmd.arg("status-line")
        .arg("--show")
        .arg("git-files")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success();
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_status_line_multiline() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("model,directory;context,cost")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("Opus 4.6"))
        .stdout(predicate::str::contains("$1.23"));
}

#[test]
fn test_status_line_empty_stdin() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line").write_stdin("").assert().failure();
}

// --- self install tests ---

fn temp_install_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir()
            .join("kozmotic-test")
            .join(format!("{}-{}", std::process::id(), name));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_self_install_json() {
    let dir = temp_install_dir("json");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("self")
        .arg("install")
        .arg("--target-dir")
        .arg(dir.as_os_str())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""))
        .stdout(predicate::str::contains("installed_path"))
        .stdout(predicate::str::contains("hook_example"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_self_install_human() {
    let dir = temp_install_dir("human");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("--format")
        .arg("human")
        .arg("self")
        .arg("install")
        .arg("--target-dir")
        .arg(dir.as_os_str())
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed to"))
        .stdout(predicate::str::contains("agent-ping"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_self_install_creates_binary() {
    let dir = temp_install_dir("binary");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("self")
        .arg("install")
        .arg("--target-dir")
        .arg(dir.as_os_str())
        .assert()
        .success();

    let binary_name = if cfg!(windows) {
        "kozmotic.exe"
    } else {
        "kozmotic"
    };
    assert!(dir.join(binary_name).exists());
    let _ = std::fs::remove_dir_all(&dir);
}
