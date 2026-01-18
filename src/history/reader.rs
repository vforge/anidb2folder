use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::types::*;
use super::writer::HistoryError;

/// Read and parse a history file
pub fn read_history(path: &Path) -> Result<HistoryFile, HistoryError> {
    let file = File::open(path)
        .map_err(|e| HistoryError::ReadError(format!("Cannot open file: {}", e)))?;

    let reader = BufReader::new(file);
    let history: HistoryFile = serde_json::from_reader(reader)
        .map_err(|e| HistoryError::ReadError(format!("Invalid JSON: {}", e)))?;

    // Version check
    if history.version != HISTORY_VERSION {
        return Err(HistoryError::VersionMismatch {
            expected: HISTORY_VERSION.to_string(),
            found: history.version,
        });
    }

    Ok(history)
}

/// Validate that a history file can be used for revert
pub fn validate_for_revert(history: &HistoryFile, target_dir: &Path) -> Result<(), HistoryError> {
    // Check target directory matches
    if history.target_directory != target_dir {
        return Err(HistoryError::ReadError(format!(
            "History file is for different directory: {:?}",
            history.target_directory
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn create_test_history() -> HistoryFile {
        HistoryFile {
            version: HISTORY_VERSION.to_string(),
            executed_at: Utc::now(),
            operation: OperationType::Rename,
            direction: HistoryDirection::AnidbToReadable,
            target_directory: PathBuf::from("/test/anime"),
            tool_version: "0.1.0".to_string(),
            changes: vec![HistoryEntry {
                source: "12345".to_string(),
                destination: "Anime (2020) [anidb-12345]".to_string(),
                anidb_id: 12345,
                truncated: false,
            }],
        }
    }

    #[test]
    fn test_read_history() {
        let dir = tempdir().unwrap();
        let history = create_test_history();
        let path = dir.path().join("test-history.json");

        // Write test file
        let content = serde_json::to_string_pretty(&history).unwrap();
        fs::write(&path, content).unwrap();

        // Read it back
        let loaded = read_history(&path).unwrap();

        assert_eq!(loaded.version, HISTORY_VERSION);
        assert_eq!(loaded.changes.len(), 1);
        assert_eq!(loaded.changes[0].anidb_id, 12345);
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_history(Path::new("/nonexistent/file.json"));
        assert!(matches!(result, Err(HistoryError::ReadError(_))));
    }

    #[test]
    fn test_read_invalid_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.json");

        fs::write(&path, "not valid json {{{").unwrap();

        let result = read_history(&path);
        assert!(matches!(result, Err(HistoryError::ReadError(_))));
    }

    #[test]
    fn test_version_mismatch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("old-version.json");

        let bad_json = r#"{
            "version": "99.0",
            "executed_at": "2026-01-01T00:00:00Z",
            "operation": "rename",
            "direction": "anidb_to_readable",
            "target_directory": "/test",
            "tool_version": "0.1.0",
            "changes": []
        }"#;
        fs::write(&path, bad_json).unwrap();

        let result = read_history(&path);
        assert!(matches!(
            result,
            Err(HistoryError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn test_validate_for_revert_success() {
        let history = create_test_history();
        let result = validate_for_revert(&history, Path::new("/test/anime"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_for_revert_wrong_directory() {
        let history = create_test_history();
        let result = validate_for_revert(&history, Path::new("/different/path"));
        assert!(matches!(result, Err(HistoryError::ReadError(_))));
    }
}
