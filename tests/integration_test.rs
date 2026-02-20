use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn test_help() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent-friendly CLI tools"));
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
        .stdout(predicate::str::contains("PostToolUse"))
        .stdout(predicate::str::contains("Stop"))
        .stdout(predicate::str::contains("SubagentStop"))
        .stdout(predicate::str::contains("TaskCompleted"))
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
        .stdout(predicate::str::contains("PostToolUse"))
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
