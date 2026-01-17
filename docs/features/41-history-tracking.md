# 41 - History Tracking

## Summary

Log all rename operations to a JSON history file for audit trail and revert capability.

## Dependencies

- **20-rename-to-readable** — Provides rename results to log
- **21-rename-to-anidb** — Provides rename results to log

## Description

This feature implements a JSON-based history logging system that records all rename operations. Each execution creates a timestamped history file in the target directory, enabling:

- Audit trail of all changes
- Full revert capability (feature 42)
- Debugging and troubleshooting
- Change verification

## Requirements

### Functional Requirements

1. Create history file after each successful rename operation
2. Filename format: `anidb2folder-history-YYYYMMDD-HHMMSS.json`
3. Store in the target directory
4. Record for each operation:
   - Source directory path
   - Destination directory path
   - AniDB ID
   - Reason for change
   - Whether truncation occurred
5. Include metadata:
   - Timestamp of execution
   - Direction of rename
   - Tool version
   - Total operation count
6. Human-readable JSON (pretty-printed)

### Non-Functional Requirements

1. Atomic file writes
2. Valid JSON structure
3. Include schema version for future compatibility
4. Do not overwrite existing history files

## Implementation Guide

### Step 1: Define History Types

```rust
// src/history/types.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const HISTORY_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryFile {
    /// Schema version for compatibility
    pub version: String,
    
    /// When the operation was executed
    pub executed_at: DateTime<Utc>,
    
    /// Type of operation performed
    pub operation: OperationType,
    
    /// Direction of rename
    pub direction: HistoryDirection,
    
    /// Target directory path
    pub target_directory: PathBuf,
    
    /// Tool version that created this history
    pub tool_version: String,
    
    /// All changes made
    pub changes: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Rename,
    Revert,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HistoryDirection {
    AnidbToReadable,
    ReadableToAnidb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Original directory name
    pub source: String,
    
    /// New directory name
    pub destination: String,
    
    /// AniDB ID for the anime
    pub anidb_id: u32,
    
    /// Why this change was made
    pub reason: String,
    
    /// Whether the name was truncated
    pub truncated: bool,
}

impl HistoryFile {
    /// Generate the filename for this history file
    pub fn generate_filename(&self) -> String {
        let timestamp = self.executed_at.format("%Y%m%d-%H%M%S");
        format!("anidb2folder-history-{}.json", timestamp)
    }
    
    /// Generate filename for a revert of this history
    pub fn generate_revert_filename(&self) -> String {
        let original_timestamp = self.executed_at.format("%Y%m%d-%H%M%S");
        let revert_timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        format!(
            "anidb2folder-history-{}-revert-{}.json",
            original_timestamp,
            revert_timestamp
        )
    }
}
```

### Step 2: Implement History Writer

```rust
// src/history/writer.rs
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use chrono::Utc;
use tracing::{info, warn};

use crate::rename::{RenameDirection, RenameResult};
use super::types::*;

/// Write history file for a rename operation
pub fn write_history(
    result: &RenameResult,
    target_dir: &Path,
) -> Result<PathBuf, HistoryError> {
    let history = create_history_from_result(result, target_dir);
    write_history_file(&history, target_dir)
}

fn create_history_from_result(result: &RenameResult, target_dir: &Path) -> HistoryFile {
    let direction = match result.direction {
        RenameDirection::AniDbToReadable => HistoryDirection::AnidbToReadable,
        RenameDirection::ReadableToAnidb => HistoryDirection::ReadableToAnidb,
    };
    
    let changes: Vec<HistoryEntry> = result.operations.iter().map(|op| {
        HistoryEntry {
            source: op.source_name.clone(),
            destination: op.destination_name.clone(),
            anidb_id: op.anidb_id,
            reason: op.reason.clone(),
            truncated: op.truncated,
        }
    }).collect();
    
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

fn write_history_file(history: &HistoryFile, target_dir: &Path) -> Result<PathBuf, HistoryError> {
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
```

### Step 3: Implement History Reader

```rust
// src/history/reader.rs
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::types::*;
use super::writer::HistoryError;

/// Read and parse a history file
pub fn read_history(path: &Path) -> Result<HistoryFile, HistoryError> {
    let file = File::open(path).map_err(|e| {
        HistoryError::ReadError(format!("Cannot open file: {}", e))
    })?;
    
    let reader = BufReader::new(file);
    let history: HistoryFile = serde_json::from_reader(reader).map_err(|e| {
        HistoryError::ReadError(format!("Invalid JSON: {}", e))
    })?;
    
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
```

### Step 4: Module Organization

```rust
// src/history/mod.rs
mod types;
mod writer;
mod reader;

pub use types::*;
pub use writer::{write_history, HistoryError};
pub use reader::{read_history, validate_for_revert};
```

### Step 5: Integration with Main

```rust
// src/main.rs (history integration)
use crate::history::write_history;

fn main() -> Result<()> {
    // ... rename operations ...
    
    if !args.dry {
        // Write history file
        let history_path = write_history(&result, target_dir)?;
        info!("History saved to: {:?}", history_path);
    }
    
    Ok(())
}
```

## Example History File

```json
{
  "version": "1.0",
  "executed_at": "2026-01-15T10:30:45.123456Z",
  "operation": "rename",
  "direction": "anidb_to_readable",
  "target_directory": "/home/user/anime",
  "tool_version": "0.1.0",
  "changes": [
    {
      "source": "[AS0] 1",
      "destination": "[AS0] Cowboyu Bebopu ／ Cowboy Bebop (1998) [anidb-1]",
      "anidb_id": 1,
      "reason": "Converted from AniDB format to human-readable format",
      "truncated": false
    },
    {
      "source": "12345",
      "destination": "Naruto (2002) [anidb-12345]",
      "anidb_id": 12345,
      "reason": "Converted from AniDB format to human-readable format",
      "truncated": false
    }
  ]
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    fn create_test_history() -> HistoryFile {
        HistoryFile {
            version: HISTORY_VERSION.to_string(),
            executed_at: Utc::now(),
            operation: OperationType::Rename,
            direction: HistoryDirection::AnidbToReadable,
            target_directory: PathBuf::from("/test"),
            tool_version: "0.1.0".to_string(),
            changes: vec![
                HistoryEntry {
                    source: "12345".to_string(),
                    destination: "Anime (2020) [anidb-12345]".to_string(),
                    anidb_id: 12345,
                    reason: "Test".to_string(),
                    truncated: false,
                },
            ],
        }
    }
    
    #[test]
    fn test_generate_filename() {
        let history = create_test_history();
        let filename = history.generate_filename();
        
        assert!(filename.starts_with("anidb2folder-history-"));
        assert!(filename.ends_with(".json"));
    }
    
    #[test]
    fn test_write_and_read_history() {
        let dir = tempdir().unwrap();
        let history = create_test_history();
        
        let path = write_history_file(&history, dir.path()).unwrap();
        assert!(path.exists());
        
        let loaded = read_history(&path).unwrap();
        assert_eq!(loaded.version, history.version);
        assert_eq!(loaded.changes.len(), 1);
    }
    
    #[test]
    fn test_pretty_printed_json() {
        let dir = tempdir().unwrap();
        let history = create_test_history();
        
        let path = write_history_file(&history, dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        
        // Pretty printed JSON should have newlines
        assert!(content.contains('\n'));
        assert!(content.contains("  ")); // Indentation
    }
    
    #[test]
    fn test_version_mismatch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        
        // Write with wrong version
        let bad_json = r#"{"version": "99.0", "executed_at": "2026-01-01T00:00:00Z"}"#;
        fs::write(&path, bad_json).unwrap();
        
        let result = read_history(&path);
        assert!(matches!(result, Err(HistoryError::VersionMismatch { .. })));
    }
}
```

### Integration Tests

```rust
// tests/history_integration_tests.rs

#[test]
fn test_history_after_rename() {
    // Full integration test: rename and verify history
}

#[test]
fn test_history_file_location() {
    // Verify history is written to correct directory
}
```

## Notes

- History files are never overwritten — each execution creates a new file
- The atomic write pattern prevents corrupt files on crash
- Version field allows future format migrations
- Consider adding compression for large histories in the future
- History files can be used for bulk revert (feature 42)
