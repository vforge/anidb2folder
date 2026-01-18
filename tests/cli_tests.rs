use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn create_anidb_dirs(dir: &std::path::Path) {
    std::fs::create_dir(dir.join("12345")).unwrap();
    std::fs::create_dir(dir.join("[AS0] 67890")).unwrap();
}

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
    create_anidb_dirs(dir.path());

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--dry", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN"))
        .stdout(predicate::str::contains("AniDB -> Human-readable"))
        .stdout(predicate::str::contains("Planned changes"));
}

#[test]
fn test_dry_flag_no_filesystem_changes() {
    let dir = tempdir().unwrap();
    let original_name = "12345";
    std::fs::create_dir(dir.path().join(original_name)).unwrap();

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--dry", dir.path().to_str().unwrap()])
        .assert()
        .success();

    // Verify directory unchanged after dry run
    assert!(dir.path().join(original_name).exists());
}

#[test]
fn test_verbose_flag() {
    let dir = tempdir().unwrap();
    create_anidb_dirs(dir.path());

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
    create_anidb_dirs(dir.path());

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--max-length", "200", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_cache_expiry_flag() {
    let dir = tempdir().unwrap();
    create_anidb_dirs(dir.path());

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--cache-expiry", "7", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_all_flags_combined() {
    let dir = tempdir().unwrap();
    create_anidb_dirs(dir.path());

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
fn test_validates_anidb_format() {
    let dir = tempdir().unwrap();
    create_anidb_dirs(dir.path());

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["-v", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("AniDb format"));
}

#[test]
fn test_rejects_unrecognized_format() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Invalid Directory")).unwrap();

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .arg(dir.path().to_str().unwrap())
        .assert()
        .code(5) // ExitCode::UnrecognizedFormat
        .stderr(predicate::str::contains("do not match any known format"));
}

#[test]
fn test_rejects_mixed_formats() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("12345")).unwrap();
    std::fs::create_dir(dir.path().join("Naruto (2002) [anidb-67890]")).unwrap();

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .arg(dir.path().to_str().unwrap())
        .assert()
        .code(4) // ExitCode::MixedFormats
        .stderr(predicate::str::contains("multiple formats"));
}

#[test]
fn test_rejects_empty_directory() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .arg(dir.path().to_str().unwrap())
        .assert()
        .code(1) // ExitCode::GeneralError (NoDirectories)
        .stderr(predicate::str::contains("No subdirectories"));
}
