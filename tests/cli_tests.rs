use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

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
    let dir = tempdir().unwrap();
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--dry", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_verbose_flag() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--verbose", dir.path().to_str().unwrap()])
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
    let dir = tempdir().unwrap();
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--max-length", "200", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_cache_expiry_flag() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--cache-expiry", "7", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_all_flags_combined() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args([
            "--dry",
            "--verbose",
            "--max-length",
            "200",
            "--cache-expiry",
            "7",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn test_nonexistent_directory() {
    Command::cargo_bin("anidb2folder")
        .unwrap()
        .arg("/nonexistent/path")
        .assert()
        .code(3) // ExitCode::DirectoryNotFound
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_file_instead_of_directory() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("file.txt");
    std::fs::write(&file_path, "content").unwrap();

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .arg(file_path.to_str().unwrap())
        .assert()
        .code(3) // ExitCode::DirectoryNotFound (NotADirectory maps to same code)
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn test_scan_with_subdirectories() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("subdir1")).unwrap();
    std::fs::create_dir(dir.path().join("subdir2")).unwrap();

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["-v", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Found 2 subdirectories"));
}
