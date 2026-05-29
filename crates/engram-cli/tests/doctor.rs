//! Integration tests for the `engram doctor` command.

use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

/// A `Command` for the built `engram` binary.
fn engram() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("engram"))
}

/// Create a fresh git repo and run `engram init` inside it.
fn init_repo() -> TempDir {
    let tmp = TempDir::new().unwrap();

    // git init + identity so commits/signatures work
    Command::new("git")
        .args(["init", "-q"])
        .current_dir(tmp.path())
        .assert()
        .success();
    for (k, v) in [("user.name", "Test"), ("user.email", "test@example.com")] {
        Command::new("git")
            .args(["config", k, v])
            .current_dir(tmp.path())
            .assert()
            .success();
    }

    engram()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    tmp
}

#[test]
fn doctor_reports_healthy_after_init() {
    let tmp = init_repo();

    engram()
        .arg("doctor")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Repository initialized for engram",
        ))
        .stdout(predicate::str::contains("Git hooks:"))
        .stdout(predicate::str::contains("prepare-commit-msg"));
}

#[test]
fn doctor_json_has_expected_fields() {
    let tmp = init_repo();

    let output = engram()
        .args(["doctor", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["enabled"], true);
    assert!(json["git_hooks_installed"].is_array());
    assert_eq!(json["recent_errors"], 0);
    assert!(json.get("engram_count").is_some());
}

#[test]
fn doctor_fails_outside_git_repo() {
    let tmp = TempDir::new().unwrap();

    engram()
        .arg("doctor")
        .current_dir(tmp.path())
        .assert()
        .failure();
}
