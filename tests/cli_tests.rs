use assert_cmd::cargo::cargo_bin_cmd;
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
    cargo_bin_cmd!("anidb2folder")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rename anime directories"));
}

#[test]
fn test_version_flag() {
    cargo_bin_cmd!("anidb2folder")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_missing_target_dir() {
    cargo_bin_cmd!("anidb2folder")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_dry_flag() {
    let dir = tempdir().unwrap();
    setup_anidb_test(dir.path());

    cargo_bin_cmd!("anidb2folder")
        .args(["--dry", dir.path().to_str().unwrap()])
        .assert()
        .success()
        // UI output goes to stderr
        .stderr(predicate::str::contains("DRY RUN"))
        .stderr(predicate::str::contains("Human-readable"))
        .stderr(predicate::str::contains("would be renamed"));
}

#[test]
fn test_dry_flag_no_filesystem_changes() {
    let dir = tempdir().unwrap();
    let original_name = "12345";
    std::fs::create_dir(dir.path().join(original_name)).unwrap();
    create_test_cache(dir.path());

    cargo_bin_cmd!("anidb2folder")
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

    cargo_bin_cmd!("anidb2folder")
        .args(["--verbose", "--dry", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_revert_flag_missing_file() {
    cargo_bin_cmd!("anidb2folder")
        .args(["--revert", "/tmp/nonexistent-history.json"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Cannot open file"));
}

#[test]
fn test_max_length_flag() {
    let dir = tempdir().unwrap();
    setup_anidb_test(dir.path());

    cargo_bin_cmd!("anidb2folder")
        .args(["--dry", "--max-length", "200", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_cache_expiry_flag() {
    let dir = tempdir().unwrap();
    setup_anidb_test(dir.path());

    cargo_bin_cmd!("anidb2folder")
        .args(["--dry", "--cache-expiry", "7", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_all_flags_combined() {
    let dir = tempdir().unwrap();
    setup_anidb_test(dir.path());

    cargo_bin_cmd!("anidb2folder")
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
    cargo_bin_cmd!("anidb2folder")
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

    cargo_bin_cmd!("anidb2folder")
        .arg(file_path.to_str().unwrap())
        .assert()
        .code(3) // ExitCode::DirectoryNotFound (NotADirectory maps to same code)
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn test_validates_anidb_format() {
    let dir = tempdir().unwrap();
    setup_anidb_test(dir.path());

    cargo_bin_cmd!("anidb2folder")
        .args(["-v", "--dry", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("AniDb format"));
}

#[test]
fn test_rejects_unrecognized_format() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Invalid Directory")).unwrap();

    cargo_bin_cmd!("anidb2folder")
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

    cargo_bin_cmd!("anidb2folder")
        .arg(dir.path().to_str().unwrap())
        .assert()
        .code(4) // ExitCode::MixedFormats
        .stderr(predicate::str::contains("multiple formats"));
}

#[test]
fn test_rejects_empty_directory() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("anidb2folder")
        .arg(dir.path().to_str().unwrap())
        .assert()
        .code(1) // ExitCode::GeneralError (NoDirectories)
        .stderr(predicate::str::contains("No subdirectories"));
}

/// Create a test history file for revert tests
fn create_test_history(dir: &std::path::Path, target_dir: &std::path::Path) -> std::path::PathBuf {
    use chrono::Utc;

    #[derive(serde::Serialize)]
    struct HistoryEntry {
        source: String,
        destination: String,
        anidb_id: u32,
        truncated: bool,
    }

    #[derive(serde::Serialize)]
    struct HistoryFile {
        version: String,
        executed_at: chrono::DateTime<Utc>,
        operation: String,
        direction: String,
        target_directory: std::path::PathBuf,
        tool_version: String,
        changes: Vec<HistoryEntry>,
    }

    let history = HistoryFile {
        version: "1.0".to_string(),
        executed_at: Utc::now(),
        operation: "rename".to_string(),
        direction: "anidb_to_readable".to_string(),
        target_directory: target_dir.to_path_buf(),
        tool_version: "0.1.0".to_string(),
        changes: vec![HistoryEntry {
            source: "12345".to_string(),
            destination: "Test Anime (2020) [anidb-12345]".to_string(),
            anidb_id: 12345,
            truncated: false,
        }],
    };

    let history_path = dir.join("test-history.json");
    let content = serde_json::to_string_pretty(&history).unwrap();
    std::fs::write(&history_path, content).unwrap();

    history_path
}

#[test]
fn test_revert_with_mismatched_target_dir_fails() {
    let dir = tempdir().unwrap();
    let other_dir = tempdir().unwrap();

    // Create history file pointing to dir.path()
    let history_path = create_test_history(dir.path(), dir.path());

    // Try to revert with a different target directory
    cargo_bin_cmd!("anidb2folder")
        .args([
            "--revert",
            history_path.to_str().unwrap(),
            other_dir.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Directory mismatch"));
}

#[test]
fn test_revert_shows_target_directory() {
    let dir = tempdir().unwrap();

    // Create the renamed directory that would exist after a rename
    std::fs::create_dir(dir.path().join("Test Anime (2020) [anidb-12345]")).unwrap();

    // Create history file
    let history_path = create_test_history(dir.path(), dir.path());

    // Revert in dry-run mode should show target directory
    cargo_bin_cmd!("anidb2folder")
        .args(["--dry", "--revert", history_path.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Target directory"));
}

#[test]
fn test_revert_with_matching_target_dir_succeeds() {
    let dir = tempdir().unwrap();

    // Create the renamed directory that would exist after a rename
    std::fs::create_dir(dir.path().join("Test Anime (2020) [anidb-12345]")).unwrap();

    // Create history file
    let history_path = create_test_history(dir.path(), dir.path());

    // Revert with matching target directory should succeed (in dry-run mode)
    cargo_bin_cmd!("anidb2folder")
        .args([
            "--dry",
            "--revert",
            history_path.to_str().unwrap(),
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Target directory verified"));
}
