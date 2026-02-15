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
