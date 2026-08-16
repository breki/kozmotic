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
    // In detached HEAD (e.g. tag checkout in CI), git-branch
    // returns empty — just verify the command succeeds.
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("git-branch")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success();
}

#[test]
fn test_status_line_api_status() {
    // The exact health depends on Anthropic and on network reach, but
    // the widget must always produce a line: a vanishing api widget
    // looks identical to a healthy API, which is how an outage went
    // unreported.
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("api-status")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("api"))
        .stdout(
            predicate::str::is_match(r"ok|degraded|outage|critical|unknown")
                .expect("valid regex"),
        );
}

/// Visible columns of a rendered line: ANSI escapes occupy none.
/// A CSI sequence is `ESC [` then parameters then a final byte in
/// 0x40..=0x7E — the `[` is in that range too, so skip it first.
fn visible_width(line: &str) -> usize {
    let mut count = 0;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            count += 1;
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            for e in chars.by_ref() {
                if ('\x40'..='\x7e').contains(&e) {
                    break;
                }
            }
        } else {
            chars.next();
        }
    }
    count
}

#[test]
fn test_status_line_right_align_pads_to_width() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    let out = cmd
        .arg("status-line")
        .arg("--show")
        .arg("model~cost")
        .arg("--width")
        .arg("60")
        .write_stdin(SAMPLE_STATUS_JSON)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    let line = text.lines().next().expect("one line");
    assert_eq!(visible_width(line), 60, "line was {line:?}");
    assert!(line.starts_with("Opus 4.6"));
    assert!(line.trim_end().ends_with("$1.23"));
}

#[test]
fn test_status_line_right_align_multiline() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    let out = cmd
        .arg("status-line")
        .arg("--show")
        .arg("model~cost;context~lines")
        .arg("--width")
        .arg("50")
        .write_stdin(SAMPLE_STATUS_JSON)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        assert_eq!(visible_width(line), 50, "line was {line:?}");
    }
}

#[test]
fn test_status_line_without_marker_is_unpadded() {
    // Absent a "~", output must be exactly as before the feature.
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("model,cost")
        .arg("--width")
        .arg("60")
        .write_stdin(SAMPLE_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("Opus 4.6 | "))
        .stdout(predicate::str::contains("   ").not());
}

#[test]
fn test_status_line_right_align_overflow_does_not_truncate() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("model~cost")
        .arg("--width")
        .arg("5")
        .write_stdin(SAMPLE_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("Opus 4.6"))
        .stdout(predicate::str::contains("$1.23"));
}

#[test]
fn test_status_line_host() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("host")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        // The label carries ANSI codes, so match around them.
        .stdout(
            predicate::str::is_match(r"host\x1b\[0m \S+").expect("valid regex"),
        );
}

#[test]
fn test_status_line_ram() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("ram")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        // e.g. "ram 12.4/31.3G"
        .stdout(
            predicate::str::is_match(r"ram.*\d+(\.\d)?/\d+(\.\d)?[BKMGT]")
                .expect("valid regex"),
        );
}

#[test]
fn test_status_line_disk() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("disk")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(r"disk.*\d+(\.\d)?/\d+(\.\d)?[BKMGT]")
                .expect("valid regex"),
        );
}

#[test]
fn test_status_line_disk_uses_workspace_dir() {
    // An unknown workspace path must still resolve to some mount
    // rather than blanking the widget.
    let json = r#"{
        "model": {},
        "context_window": {},
        "cost": {},
        "workspace": { "current_dir": "/nonexistent/path/xyz" }
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("disk")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(r"disk.*\d+(\.\d)?/\d+(\.\d)?[BKMGT]")
                .expect("valid regex"),
        );
}

#[test]
fn test_status_line_git_ahead() {
    // Just check it runs without error - actual counts depend on repo state
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("git-ahead")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success();
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
fn test_status_line_cost_rate() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    // cost=$1.23, duration=754s ≈ 0.2094h, rate ≈ $5.87/h
    cmd.arg("status-line")
        .arg("--show")
        .arg("cost-rate")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("$5.87/h"));
}

#[test]
fn test_status_line_cost_rate_zero_duration() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("cost-rate")
        .write_stdin(r#"{"cost":{"total_cost_usd":1.0,"total_duration_ms":0}}"#)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_status_line_last_commit() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("last-commit")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success();
}

#[test]
fn test_status_line_git_lines() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("git-lines")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success();
}

#[test]
fn test_status_line_lines() {
    // The "lines" widget renders +added/-removed from cost data.
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("lines")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success()
        .stdout(predicate::str::contains("+150"))
        .stdout(predicate::str::contains("-30"));
}

#[test]
fn test_status_line_context_yellow() {
    // 50-80% used should colour the context widget yellow.
    let json = r#"{
        "model": {},
        "context_window": { "used_percentage": 65.0 },
        "cost": {}
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("context")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[33m")); // yellow
}

#[test]
fn test_status_line_rate_limit_7d() {
    let json = r#"{
        "model": {},
        "context_window": {},
        "cost": {},
        "rate_limits": {
            "seven_day": { "used_percentage": 12.0 }
        }
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("rate-limit-7d")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("12%"));
}

#[test]
fn test_status_line_rate_limit_with_reset() {
    // resets_at as Unix timestamp triggers the format_reset
    // path that appends "(→HH:MM)".
    let json = r#"{
        "rate_limits": {
            "five_hour": {
                "used_percentage": 73.2,
                "resets_at": 1735689600
            }
        }
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("rate-limit")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("73%"))
        .stdout(predicate::str::contains("→"));
}

#[test]
fn test_status_line_rate_limit_rfc3339_reset() {
    // Resets_at as RFC3339 string -- exercises parse_rfc3339.
    let json = r#"{
        "rate_limits": {
            "five_hour": {
                "used_percentage": 50.0,
                "resets_at": "2026-04-20T15:04:05Z"
            }
        }
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("rate-limit")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("50%"));
}

#[test]
fn test_status_line_invalid_json() {
    // Bad JSON should print a diagnostic, not crash.
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .write_stdin("{not valid json")
        .assert()
        .failure()
        .stdout(predicate::str::contains("status-line"));
}

#[test]
fn test_status_line_unknown_widget() {
    // Unknown widget names render to nothing.
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("nonsense-widget")
        .write_stdin(FULL_STATUS_JSON)
        .assert()
        .success();
}

#[test]
fn test_status_line_duration_hours() {
    // Past the 1-hour mark, duration switches to "Xh Ym".
    let json = r#"{
        "cost": { "total_duration_ms": 4500000 }
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("duration")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("1h 15m"));
}

#[test]
fn test_status_line_duration_days() {
    // Past 24 hours, duration switches to "Xd Yh".
    let json = r#"{
        "cost": { "total_duration_ms": 180000000 }
    }"#;
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("status-line")
        .arg("--show")
        .arg("duration")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("2d"));
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

// --- agent-ping muted ---

fn fake_home_with_mute(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("kozmotic-test-home")
        .join(format!("{}-{}", std::process::id(), name));
    let claude = dir.join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::fs::write(claude.join(".mute-sounds"), "").unwrap();
    dir
}

#[test]
fn test_agent_ping_muted_json() {
    let home = fake_home_with_mute("ping-mute-json");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("HOME", &home)
        .env("USERPROFILE", &home)
        .arg("agent-ping")
        .arg("--sound")
        .arg("Stop")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"muted\": true"))
        .stdout(predicate::str::contains("\"played\": false"));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn test_agent_ping_muted_human() {
    let home = fake_home_with_mute("ping-mute-human");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("HOME", &home)
        .env("USERPROFILE", &home)
        .arg("--format")
        .arg("human")
        .arg("agent-ping")
        .arg("--sound")
        .arg("Stop")
        .assert()
        .success()
        .stdout(predicate::str::contains("muted"));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn test_agent_ping_muted_with_file() {
    // Muted path includes "unknown" fallback when no source given
    // and uses --file source for the muted JSON output.
    let home = fake_home_with_mute("ping-mute-file");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("HOME", &home)
        .env("USERPROFILE", &home)
        .arg("agent-ping")
        .arg("--file")
        .arg("nonexistent.wav")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"muted\": true"))
        .stdout(predicate::str::contains("nonexistent.wav"));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn test_agent_ping_play_success_sound() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("KOZMOTIC_TEST_AUDIO", "ok")
        .arg("agent-ping")
        .arg("--sound")
        .arg("Stop")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"played\": true"));
}

#[test]
fn test_agent_ping_play_success_frequency() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("KOZMOTIC_TEST_AUDIO", "ok")
        .arg("agent-ping")
        .arg("--frequency")
        .arg("440")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"played\": true"))
        .stdout(predicate::str::contains("\"frequency\""));
}

#[test]
fn test_agent_ping_play_success_human() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("KOZMOTIC_TEST_AUDIO", "ok")
        .arg("--format")
        .arg("human")
        .arg("agent-ping")
        .arg("--sound")
        .arg("Stop")
        .assert()
        .success()
        .stdout(predicate::str::contains("Played: Stop"));
}

#[test]
fn test_agent_ping_play_success_file() {
    // Use an existing file (the binary itself) as a stand-in;
    // playback is overridden so the file is never decoded.
    let exe = std::env::current_exe().unwrap();
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("KOZMOTIC_TEST_AUDIO", "ok")
        .arg("agent-ping")
        .arg("--file")
        .arg(exe.as_os_str())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"played\": true"));
}

#[test]
fn test_agent_ping_play_audio_device_error() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("KOZMOTIC_TEST_AUDIO", "err")
        .arg("agent-ping")
        .arg("--sound")
        .arg("Stop")
        .assert()
        .failure()
        .stderr(predicate::str::contains("AUDIO_DEVICE_ERROR"));
}

// --- self install tests ---

fn temp_install_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("kozmotic-test").join(format!(
        "{}-{}",
        std::process::id(),
        name
    ));
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
fn test_self_install_home_not_found() {
    // With both HOME and USERPROFILE unset and no --target-dir,
    // home_dir() returns None and the error path fires.
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env_remove("HOME")
        .env_remove("USERPROFILE")
        .arg("self")
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("HOME_NOT_FOUND"));
}

#[test]
fn test_self_install_home_not_found_human() {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env_remove("HOME")
        .env_remove("USERPROFILE")
        .arg("--format")
        .arg("human")
        .arg("self")
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("HOME_NOT_FOUND"));
}

#[test]
fn test_self_install_create_dir_fails() {
    // Point --target-dir at a path that exists as a *file*, so
    // create_dir_all fails.
    let blocker = std::env::temp_dir()
        .join(format!("kozmotic-blocker-{}", std::process::id()));
    std::fs::write(&blocker, "x").unwrap();
    let target = blocker.join("nested");

    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.arg("self")
        .arg("install")
        .arg("--target-dir")
        .arg(target.as_os_str())
        .assert()
        .failure()
        .stderr(predicate::str::contains("CREATE_DIR"));
    let _ = std::fs::remove_file(&blocker);
}

#[test]
fn test_self_install_no_home_tilde_path() {
    // When HOME is unset but --target-dir is given, install
    // succeeds but the tilde-substituted path falls back to
    // the literal install path.
    let dir = temp_install_dir("no-home");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env_remove("HOME")
        .env_remove("USERPROFILE")
        .arg("--format")
        .arg("human")
        .arg("self")
        .arg("install")
        .arg("--target-dir")
        .arg(dir.as_os_str())
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed to"));
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

// --- sessions prompts tests ---

/// The project path our fixture transcripts are recorded under, and
/// the directory name Claude Code would give it.
const FIXTURE_PROJECT: &str = "/fixture/project";
const FIXTURE_SLUG: &str = "-fixture-project";

fn record(kind: &str, content: &str, extra: &str) -> String {
    format!(
        r#"{{"type":"{kind}","uuid":"u","timestamp":"2026-08-14T19:00:00Z",{extra}"message":{{"role":"user","content":{}}}}}"#,
        serde_json::to_string(content).unwrap()
    )
}

/// A config dir holding one project with one transcript, laid out
/// exactly as Claude Code lays out its own storage.
fn fixture_store(session: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("projects").join(FIXTURE_SLUG);
    std::fs::create_dir_all(&dir).unwrap();
    let lines = [
        record(
            "user",
            "<local-command-caveat>skip me</local-command-caveat>",
            r#""isMeta":true,"#,
        ),
        record("user", "<command-name>/commit</command-name>", ""),
        record("user", "first real prompt", ""),
        record("assistant", "not user input", ""),
        record("user", "second real prompt\nwith a second line", ""),
    ];
    std::fs::write(
        dir.join(format!("{session}.jsonl")),
        lines.join("\n") + "\n",
    )
    .unwrap();
    root
}

/// A command pointed at the fixture store, with the ambient session
/// id removed so tests do not inherit the real one when run under
/// Claude Code.
fn prompts_cmd(store: &tempfile::TempDir) -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("CLAUDE_CONFIG_DIR", store.path())
        .env_remove("CLAUDE_CODE_SESSION_ID");
    cmd
}

#[test]
fn test_sessions_prompts_json() {
    let store = fixture_store("sess-1");
    prompts_cmd(&store)
        .args(["sessions", "prompts", "--project", FIXTURE_PROJECT])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""))
        .stdout(predicate::str::contains("\"tool\": \"sessions-prompts\""))
        .stdout(predicate::str::contains("\"session_id\": \"sess-1\""))
        .stdout(predicate::str::contains("\"count\": 3"))
        .stdout(predicate::str::contains("first real prompt"))
        .stdout(predicate::str::contains("\"command\": \"/commit\""))
        .stdout(predicate::str::contains("skip me").not());
}

#[test]
fn test_sessions_prompts_human_elides_extra_lines() {
    let store = fixture_store("sess-1");
    prompts_cmd(&store)
        .args([
            "--format",
            "human",
            "sessions",
            "prompts",
            "--project",
            FIXTURE_PROJECT,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("/commit"))
        .stdout(predicate::str::contains("first real prompt"))
        .stdout(predicate::str::contains("(+1 lines)"))
        .stdout(predicate::str::contains("status").not());
}

#[test]
fn test_sessions_prompts_no_commands() {
    let store = fixture_store("sess-1");
    prompts_cmd(&store)
        .args([
            "sessions",
            "prompts",
            "--project",
            FIXTURE_PROJECT,
            "--no-commands",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\": 2"))
        .stdout(predicate::str::contains("/commit").not());
}

#[test]
fn test_sessions_prompts_limit_keeps_the_latest() {
    let store = fixture_store("sess-1");
    prompts_cmd(&store)
        .args([
            "sessions",
            "prompts",
            "--project",
            FIXTURE_PROJECT,
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\": 1"))
        .stdout(predicate::str::contains("second real prompt"))
        .stdout(predicate::str::contains("first real prompt").not())
        // --limit trims the listing but does not renumber it.
        .stdout(predicate::str::contains("\"index\": 3"));
}

#[test]
fn test_sessions_prompts_uses_ambient_session_id() {
    let store = fixture_store("sess-1");
    // No --session: the id comes from the environment, the way it
    // does for a command Claude Code runs inside a session.
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("CLAUDE_CONFIG_DIR", store.path())
        .env("CLAUDE_CODE_SESSION_ID", "sess-1")
        .args(["sessions", "prompts", "--project", "/somewhere/else"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"session_id\": \"sess-1\""));
}

#[test]
fn test_sessions_prompts_explicit_session_wins() {
    let store = fixture_store("sess-1");
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("CLAUDE_CONFIG_DIR", store.path())
        .env("CLAUDE_CODE_SESSION_ID", "not-this-one")
        .args([
            "sessions",
            "prompts",
            "--project",
            FIXTURE_PROJECT,
            "--session",
            "sess-1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"session_id\": \"sess-1\""));
}

#[test]
fn test_sessions_prompts_unknown_session() {
    let store = fixture_store("sess-1");
    prompts_cmd(&store)
        .args([
            "sessions",
            "prompts",
            "--project",
            FIXTURE_PROJECT,
            "--session",
            "nope",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SESSION_NOT_FOUND"));
}

#[test]
fn test_sessions_prompts_project_without_sessions() {
    let store = fixture_store("sess-1");
    prompts_cmd(&store)
        .args([
            "--format",
            "human",
            "sessions",
            "prompts",
            "--project",
            "/no/such/project",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NO_SESSIONS"));
}

#[test]
fn test_sessions_prompts_without_storage() {
    let empty = tempfile::tempdir().unwrap();
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("CLAUDE_CONFIG_DIR", empty.path())
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .args(["sessions", "prompts", "--project", FIXTURE_PROJECT])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NO_STORAGE"));
}

#[test]
fn test_sessions_prompts_empty_session_reports_none() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("projects").join(FIXTURE_SLUG);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("blank.jsonl"), "").unwrap();
    let mut cmd = cargo_bin_cmd!("kozmotic");
    cmd.env("CLAUDE_CONFIG_DIR", root.path())
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .args([
            "--format",
            "human",
            "sessions",
            "prompts",
            "--project",
            FIXTURE_PROJECT,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No prompts in session blank"));
}
