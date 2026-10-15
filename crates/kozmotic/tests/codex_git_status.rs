use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git is installed");
    assert!(output.status.success(), "{:?}", output.stderr);
}

fn repository() -> TempDir {
    let dir = tempfile::Builder::new()
        .prefix("kozmotic codex ")
        .tempdir()
        .unwrap();
    git(dir.path(), &["init", "--initial-branch=main"]);
    std::fs::write(dir.path().join("tracked"), "original\n").unwrap();
    git(dir.path(), &["add", "tracked"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=Hook Test",
            "-c",
            "user.email=hook-test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "fixture",
        ],
    );
    dir
}

fn invoke(cwd: &Path, input: &str) -> Value {
    let output = cargo_bin_cmd!("kozmotic")
        .arg("codex-git-status")
        .current_dir(cwd)
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("hook JSON, not an envelope")
}

#[test]
fn codex_git_status_reports_the_payload_repository_in_plain_text() {
    let repo = repository();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("tracked"), "original\nadded\n").unwrap();
    std::fs::write(repo.path().join("new"), "new\n").unwrap();
    let result =
        invoke(elsewhere.path(), &json!({"cwd": repo.path()}).to_string());
    assert_eq!(result.as_object().unwrap().len(), 1);
    let message = result["systemMessage"].as_str().unwrap();
    assert!(message.starts_with("Git: main"), "{message}");
    assert!(message.contains("1mod") && message.contains("1new"));
    assert!(message.contains("+1/-0"), "{message}");
    assert!(message.contains("last"));
    assert!(!message.chars().any(char::is_control));
    assert!(message.chars().count() <= 1000);
}

#[test]
fn codex_git_status_reports_staged_files_and_missing_upstream() {
    let repo = repository();
    std::fs::write(repo.path().join("tracked"), "changed\n").unwrap();
    git(repo.path(), &["add", "tracked"]);
    let result = invoke(repo.path(), &json!({"cwd": repo.path()}).to_string());
    let message = result["systemMessage"].as_str().unwrap();
    assert!(message.contains("1staged"), "{message}");
    assert!(message.contains("no upstream"), "{message}");
}

#[test]
fn codex_git_status_is_quiet_outside_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        invoke(dir.path(), &json!({"cwd": dir.path()}).to_string()),
        json!({})
    );
}

#[test]
fn codex_git_status_ignores_invalid_input_without_blocking() {
    let dir = tempfile::tempdir().unwrap();
    for input in [
        "",
        "not JSON",
        "[]",
        "null",
        "{}",
        r#"{"cwd":12}"#,
        r#"{"cwd":""}"#,
    ] {
        assert_eq!(invoke(dir.path(), input), json!({}), "{input}");
    }
}

#[test]
fn codex_git_status_handles_detached_head() {
    let repo = repository();
    git(repo.path(), &["checkout", "--detach"]);
    let result = invoke(repo.path(), &json!({"cwd": repo.path()}).to_string());
    let message = result["systemMessage"].as_str().unwrap();
    assert!(message.contains("last"), "{message}");
    assert!(!message.contains("main"));
}

#[test]
fn codex_git_status_protocol_does_not_depend_on_global_format() {
    let repo = repository();
    let output = cargo_bin_cmd!("kozmotic")
        .args(["--format", "human", "codex-git-status"])
        .write_stdin(json!({"cwd": repo.path()}).to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let result: Value = serde_json::from_slice(&output).unwrap();
    assert!(result["systemMessage"].is_string());
}
