use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use chrono::Utc;
use tracing::{info, warn};

use crate::rename::{RenameDirection, RenameResult};

use super::types::*;

/// Error types for history operations
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Failed to write history file: {0}")]
    WriteError(#[from] std::io::Error),

    #[error("Failed to serialize history: {0}")]
    SerializeError(#[from] serde_json::Error),

    #[error("Failed to read history file: {0}")]
    ReadError(String),

    #[error("History file version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
}

/// Write history file for a rename operation
pub fn write_history(result: &RenameResult, target_dir: &Path) -> Result<PathBuf, HistoryError> {
    let history = create_history_from_result(result, target_dir);
    write_history_file(&history, target_dir)
}

fn create_history_from_result(result: &RenameResult, target_dir: &Path) -> HistoryFile {
    let direction = match result.direction {
        RenameDirection::AniDbToReadable => HistoryDirection::AnidbToReadable,
        RenameDirection::ReadableToAniDb => HistoryDirection::ReadableToAnidb,
    };

    let changes: Vec<HistoryEntry> = result
        .operations
        .iter()
        .map(|op| HistoryEntry {
            source: op.source_name.clone(),
            destination: op.destination_name.clone(),
            anidb_id: op.anidb_id,
            truncated: op.truncated,
        })
        .collect();

    HistoryFile {
        version: HISTORY_VERSION.to_string(),
        executed_at: Utc::now(),
        operation: OperationType::Rename,
        direction,
        target_directory: target_dir.to_path_buf(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        changes,
    }
}

pub fn write_history_file(history: &HistoryFile, target_dir: &Path) -> Result<PathBuf, HistoryError> {
    let filename = history.generate_filename();
    let file_path = target_dir.join(&filename);

    // Check if file already exists (shouldn't happen, but be safe)
    if file_path.exists() {
        warn!("History file already exists: {:?}", file_path);
        // Add milliseconds to make unique
        let unique_filename = format!(
            "anidb2folder-history-{}-{}.json",
            history.executed_at.format("%Y%m%d-%H%M%S"),
            history.executed_at.timestamp_subsec_millis()
        );
        let unique_path = target_dir.join(unique_filename);
        return write_to_path(history, &unique_path);
    }

    write_to_path(history, &file_path)
}

fn write_to_path(history: &HistoryFile, path: &Path) -> Result<PathBuf, HistoryError> {
    // Write to temporary file first
    let temp_path = path.with_extension("json.tmp");

    {
        let file = File::create(&temp_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, history)?;
    }

    // Atomic rename
    fs::rename(&temp_path, path)?;

    info!("History written to: {:?}", path);

    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rename::RenameOperation;
    use tempfile::tempdir;

    fn create_test_result() -> RenameResult {
        let mut result = RenameResult::new(RenameDirection::AniDbToReadable, false);
        result.add_operation(RenameOperation::new(
            PathBuf::from("/anime/12345"),
            "Anime (2020) [anidb-12345]".to_string(),
            12345,
            false,
        ));
        result.add_operation(RenameOperation::new(
            PathBuf::from("/anime/67890"),
            "Another Anime… [anidb-67890]".to_string(),
            67890,
            true,
        ));
        result
    }

    #[test]
    fn test_write_history() {
        let dir = tempdir().unwrap();
        let result = create_test_result();

        let path = write_history(&result, dir.path()).unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().contains("anidb2folder-history-"));
        assert!(path.to_string_lossy().ends_with(".json"));
    }

    #[test]
    fn test_history_content() {
        let dir = tempdir().unwrap();
        let result = create_test_result();

        let path = write_history(&result, dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        // Verify it's valid JSON
        let history: HistoryFile = serde_json::from_str(&content).unwrap();

        assert_eq!(history.version, HISTORY_VERSION);
        assert_eq!(history.operation, OperationType::Rename);
        assert_eq!(history.direction, HistoryDirection::AnidbToReadable);
        assert_eq!(history.changes.len(), 2);
        assert_eq!(history.changes[0].anidb_id, 12345);
        assert!(!history.changes[0].truncated);
        assert!(history.changes[1].truncated);
    }

    #[test]
    fn test_pretty_printed_json() {
        let dir = tempdir().unwrap();
        let result = create_test_result();

        let path = write_history(&result, dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        // Pretty printed JSON should have newlines and indentation
        assert!(content.contains('\n'));
        assert!(content.contains("  ")); // Indentation
    }

    #[test]
    fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let result = create_test_result();

        let path = write_history(&result, dir.path()).unwrap();

        // Temp file should not exist after write
        let temp_path = path.with_extension("json.tmp");
        assert!(!temp_path.exists());
    }
}
