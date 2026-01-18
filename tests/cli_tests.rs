use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn create_anidb_dirs(dir: &std::path::Path) {
    std::fs::create_dir(dir.join("12345")).unwrap();
    std::fs::create_dir(dir.join("[AS0] 67890")).unwrap();
}

/// Create a cache file with test data so tests don't need API calls
fn create_test_cache(dir: &std::path::Path) {
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;

    // Cache structure matches CacheFile from cache/types.rs
    #[derive(serde::Serialize)]
    struct CacheEntry {
        anidb_id: u32,
        title_main: String,
        title_en: Option<String>,
        release_year: Option<u16>,
        fetched_at: DateTime<Utc>,
    }

    #[derive(serde::Serialize)]
    struct CacheFile {
        version: String,
        entries: HashMap<u32, CacheEntry>,
    }

    let now = Utc::now();
    let mut entries = HashMap::new();

    entries.insert(
        12345,
        CacheEntry {
            anidb_id: 12345,
            title_main: "Test Anime".to_string(),
            title_en: Some("Test Anime English".to_string()),
            release_year: Some(2020),
            fetched_at: now,
        },
    );

    entries.insert(
        67890,
        CacheEntry {
            anidb_id: 67890,
            title_main: "Another Anime".to_string(),
            title_en: None,
            release_year: Some(2021),
            fetched_at: now,
        },
    );

    let cache = CacheFile {
        version: "1.0".to_string(),
        entries,
    };

    let cache_path = dir.join(".anidb2folder-cache.json");
    let content = serde_json::to_string_pretty(&cache).unwrap();
    std::fs::write(cache_path, content).unwrap();
}

/// Create AniDB dirs and pre-populate cache
fn setup_anidb_test(dir: &std::path::Path) {
    create_anidb_dirs(dir);
    create_test_cache(dir);
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
    setup_anidb_test(dir.path());

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
    create_test_cache(dir.path());

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
    setup_anidb_test(dir.path());

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--verbose", "--dry", dir.path().to_str().unwrap()])
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
    setup_anidb_test(dir.path());

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--dry", "--max-length", "200", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_cache_expiry_flag() {
    let dir = tempdir().unwrap();
    setup_anidb_test(dir.path());

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["--dry", "--cache-expiry", "7", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_all_flags_combined() {
    let dir = tempdir().unwrap();
    setup_anidb_test(dir.path());

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
    setup_anidb_test(dir.path());

    Command::cargo_bin("anidb2folder")
        .unwrap()
        .args(["-v", "--dry", dir.path().to_str().unwrap()])
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
