use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rename anime directories"));
}

#[test]
fn test_version_flag() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_missing_target_dir() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_dry_flag() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--dry", "/tmp/test"])
        .assert()
        .success();
}

#[test]
fn test_verbose_flag() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--verbose", "/tmp/test"])
        .assert()
        .success();
}

#[test]
fn test_revert_flag_without_target() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--revert", "/tmp/history.json"])
        .assert()
        .success();
}

#[test]
fn test_max_length_flag() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--max-length", "200", "/tmp/test"])
        .assert()
        .success();
}

#[test]
fn test_cache_expiry_flag() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--cache-expiry", "7", "/tmp/test"])
        .assert()
        .success();
}

#[test]
fn test_all_flags_combined() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args([
            "--dry",
            "--verbose",
            "--max-length",
            "200",
            "--cache-expiry",
            "7",
            "/tmp/test",
        ])
        .assert()
        .success();
}
