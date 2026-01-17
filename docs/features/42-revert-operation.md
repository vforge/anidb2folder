# 42 - Revert Operation

## Summary

Restore directories to their original names using a history file.

## Dependencies

- **41-history-tracking** — Requires history file format and reader

## Description

This feature implements the revert functionality that restores directories to their previous names using a history file. When executed with `--revert`, the tool reads the specified history file and reverses all recorded changes.

The revert process:

1. Reads and validates the history file
2. Verifies all destination directories exist (current state)
3. Reverses each rename operation
4. Creates a new history file documenting the revert

## Requirements

### Functional Requirements

1. Accept history file path via `--revert` or `-r` flag
2. Validate history file format and version
3. Verify target directory matches history
4. Check all directories to revert exist
5. Reverse each rename operation
6. Create revert history file: `<original>-revert-YYYYMMDD-HHMMSS.json`
7. Support dry run mode for revert preview

### Non-Functional Requirements

1. Atomic all-or-nothing revert (no partial reverts)
2. Clear error messages for missing directories
3. Same exit codes as normal operations

## Implementation Guide

### Step 1: Implement Revert Logic

```rust
// src/revert/mod.rs
use std::fs;
use std::path::Path;
use chrono::Utc;
use tracing::{debug, error, info, warn};

use crate::history::{
    HistoryDirection, HistoryEntry, HistoryError, HistoryFile,
    OperationType, read_history, validate_for_revert, write_history,
    HISTORY_VERSION,
};
use crate::rename::{RenameDirection, RenameOperation, RenameResult};

#[derive(Debug, thiserror::Error)]
pub enum RevertError {
    #[error("History error: {0}")]
    History(#[from] HistoryError),
    
    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),
    
    #[error("Source directory still exists: {0}")]
    SourceStillExists(String),
    
    #[error("Failed to rename: {0}")]
    RenameError(String),
    
    #[error("Target directory mismatch")]
    TargetMismatch,
}

pub struct RevertOptions {
    pub dry_run: bool,
}

impl Default for RevertOptions {
    fn default() -> Self {
        Self { dry_run: false }
    }
}

/// Execute a revert operation using a history file
pub fn revert_from_history(
    history_path: &Path,
    options: &RevertOptions,
) -> Result<RevertResult, RevertError> {
    info!("Loading history from: {:?}", history_path);
    
    // Read history file
    let history = read_history(history_path)?;
    
    info!(
        "History contains {} changes from {}",
        history.changes.len(),
        history.executed_at
    );
    
    // Prepare revert operations
    let target_dir = &history.target_directory;
    let operations = prepare_revert_operations(&history, target_dir)?;
    
    // Execute reverts (unless dry run)
    if !options.dry_run {
        execute_reverts(&operations)?;
        
        // Write revert history
        let revert_history = create_revert_history(&history, &operations);
        let filename = history.generate_revert_filename();
        let revert_path = target_dir.join(&filename);
        
        write_revert_history(&revert_history, &revert_path)?;
        
        info!("Revert history saved to: {:?}", revert_path);
    }
    
    // Determine reversed direction
    let direction = match history.direction {
        HistoryDirection::AnidbToReadable => RenameDirection::ReadableToAniDb,
        HistoryDirection::ReadableToAnidb => RenameDirection::AniDbToReadable,
    };
    
    Ok(RevertResult {
        operations,
        direction,
        original_history: history_path.to_path_buf(),
    })
}

fn prepare_revert_operations(
    history: &HistoryFile,
    target_dir: &Path,
) -> Result<Vec<RevertOperation>, RevertError> {
    let mut operations = Vec::with_capacity(history.changes.len());
    let mut errors = Vec::new();
    
    for entry in &history.changes {
        // For revert: source becomes destination, destination becomes source
        let current_path = target_dir.join(&entry.destination);
        let revert_path = target_dir.join(&entry.source);
        
        // Check current (destination) exists
        if !current_path.exists() {
            errors.push(format!(
                "Expected directory not found: {} (was renamed to this)",
                entry.destination
            ));
            continue;
        }
        
        // Check original (source) doesn't exist
        if revert_path.exists() {
            errors.push(format!(
                "Original directory already exists: {} (cannot revert)",
                entry.source
            ));
            continue;
        }
        
        operations.push(RevertOperation {
            current_path,
            current_name: entry.destination.clone(),
            revert_path,
            revert_name: entry.source.clone(),
            anidb_id: entry.anidb_id,
        });
    }
    
    if !errors.is_empty() {
        error!("Revert validation failed:");
        for err in &errors {
            error!("  - {}", err);
        }
        return Err(RevertError::DirectoryNotFound(errors.join("; ")));
    }
    
    Ok(operations)
}

fn execute_reverts(operations: &[RevertOperation]) -> Result<(), RevertError> {
    for op in operations {
        info!("Reverting: {} -> {}", op.current_name, op.revert_name);
        
        fs::rename(&op.current_path, &op.revert_path).map_err(|e| {
            RevertError::RenameError(format!(
                "Failed to rename '{}' to '{}': {}",
                op.current_name, op.revert_name, e
            ))
        })?;
    }
    
    Ok(())
}

fn create_revert_history(
    original: &HistoryFile,
    operations: &[RevertOperation],
) -> HistoryFile {
    let reversed_direction = match original.direction {
        HistoryDirection::AnidbToReadable => HistoryDirection::ReadableToAnidb,
        HistoryDirection::ReadableToAnidb => HistoryDirection::AnidbToReadable,
    };
    
    let changes: Vec<HistoryEntry> = operations.iter().map(|op| {
        HistoryEntry {
            source: op.current_name.clone(),
            destination: op.revert_name.clone(),
            anidb_id: op.anidb_id,
            reason: format!(
                "Reverted from history {}",
                original.executed_at.format("%Y-%m-%d %H:%M:%S")
            ),
            truncated: false,
        }
    }).collect();
    
    HistoryFile {
        version: HISTORY_VERSION.to_string(),
        executed_at: Utc::now(),
        operation: OperationType::Revert,
        direction: reversed_direction,
        target_directory: original.target_directory.clone(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        changes,
    }
}

fn write_revert_history(history: &HistoryFile, path: &Path) -> Result<(), RevertError> {
    let temp_path = path.with_extension("json.tmp");
    
    {
        let file = fs::File::create(&temp_path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, history)?;
    }
    
    fs::rename(&temp_path, path)?;
    
    Ok(())
}

#[derive(Debug)]
pub struct RevertOperation {
    pub current_path: std::path::PathBuf,
    pub current_name: String,
    pub revert_path: std::path::PathBuf,
    pub revert_name: String,
    pub anidb_id: u32,
}

#[derive(Debug)]
pub struct RevertResult {
    pub operations: Vec<RevertOperation>,
    pub direction: RenameDirection,
    pub original_history: std::path::PathBuf,
}
```

### Step 2: Add to Main

```rust
// src/main.rs (revert integration)
use crate::revert::{revert_from_history, RevertOptions, RevertError};

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Handle revert mode
    if let Some(history_path) = &args.revert {
        let options = RevertOptions {
            dry_run: args.dry,
        };
        
        let result = revert_from_history(history_path, &options)?;
        
        if args.dry {
            display_revert_dry_run(&result);
        } else {
            info!("Successfully reverted {} directories", result.operations.len());
        }
        
        return Ok(());
    }
    
    // Normal rename operation...
}

fn display_revert_dry_run(result: &RevertResult) {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    REVERT DRY RUN                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    println!("History file: {:?}\n", result.original_history);
    println!("Planned reverts:\n");
    
    for (i, op) in result.operations.iter().enumerate() {
        println!("  {}. [anidb-{}]", i + 1, op.anidb_id);
        println!("     From: {}", op.current_name);
        println!("     To:   {}", op.revert_name);
        println!();
    }
    
    println!("Run without --dry to apply these reverts.");
}
```

### Step 3: Output Module Addition

```rust
// src/output/revert.rs
use crate::revert::RevertResult;
use std::io::{self, Write};

pub fn display_revert_results(result: &RevertResult, writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer, "\n═══════════════════════════════════════════════════")?;
    writeln!(writer, "                  REVERT COMPLETE")?;
    writeln!(writer, "═══════════════════════════════════════════════════\n")?;
    
    writeln!(writer, "Reverted {} directories\n", result.operations.len())?;
    
    for op in &result.operations {
        writeln!(writer, "  ✓ {} -> {}", op.current_name, op.revert_name)?;
    }
    
    writeln!(writer)?;
    
    Ok(())
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    
    fn setup_test_scenario() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        
        // Create "renamed" directories (as if rename happened)
        fs::create_dir(dir.path().join("Anime Title (2020) [anidb-12345]")).unwrap();
        fs::create_dir(dir.path().join("[X] Other Title (2019) [anidb-99]")).unwrap();
        
        // Create history file
        let history = HistoryFile {
            version: HISTORY_VERSION.to_string(),
            executed_at: Utc::now(),
            operation: OperationType::Rename,
            direction: HistoryDirection::AnidbToReadable,
            target_directory: dir.path().to_path_buf(),
            tool_version: "0.1.0".to_string(),
            changes: vec![
                HistoryEntry {
                    source: "12345".to_string(),
                    destination: "Anime Title (2020) [anidb-12345]".to_string(),
                    anidb_id: 12345,
                    reason: "Test".to_string(),
                    truncated: false,
                },
                HistoryEntry {
                    source: "[X] 99".to_string(),
                    destination: "[X] Other Title (2019) [anidb-99]".to_string(),
                    anidb_id: 99,
                    reason: "Test".to_string(),
                    truncated: false,
                },
            ],
        };
        
        let history_path = dir.path().join("anidb2folder-history-20260115-100000.json");
        let file = fs::File::create(&history_path).unwrap();
        serde_json::to_writer_pretty(file, &history).unwrap();
        
        (dir, history_path)
    }
    
    #[test]
    fn test_revert_success() {
        let (dir, history_path) = setup_test_scenario();
        
        let options = RevertOptions { dry_run: false };
        let result = revert_from_history(&history_path, &options).unwrap();
        
        assert_eq!(result.operations.len(), 2);
        
        // Verify directories were reverted
        assert!(dir.path().join("12345").exists());
        assert!(dir.path().join("[X] 99").exists());
        
        // Verify original names are gone
        assert!(!dir.path().join("Anime Title (2020) [anidb-12345]").exists());
        assert!(!dir.path().join("[X] Other Title (2019) [anidb-99]").exists());
    }
    
    #[test]
    fn test_revert_dry_run() {
        let (dir, history_path) = setup_test_scenario();
        
        let options = RevertOptions { dry_run: true };
        let result = revert_from_history(&history_path, &options).unwrap();
        
        assert_eq!(result.operations.len(), 2);
        
        // Verify directories are NOT changed (dry run)
        assert!(dir.path().join("Anime Title (2020) [anidb-12345]").exists());
        assert!(!dir.path().join("12345").exists());
    }
    
    #[test]
    fn test_revert_missing_directory() {
        let dir = tempdir().unwrap();
        
        // Create history but NO directories
        let history = HistoryFile {
            version: HISTORY_VERSION.to_string(),
            executed_at: Utc::now(),
            operation: OperationType::Rename,
            direction: HistoryDirection::AnidbToReadable,
            target_directory: dir.path().to_path_buf(),
            tool_version: "0.1.0".to_string(),
            changes: vec![HistoryEntry {
                source: "12345".to_string(),
                destination: "Missing Dir [anidb-12345]".to_string(),
                anidb_id: 12345,
                reason: "Test".to_string(),
                truncated: false,
            }],
        };
        
        let history_path = dir.path().join("test-history.json");
        let file = fs::File::create(&history_path).unwrap();
        serde_json::to_writer_pretty(file, &history).unwrap();
        
        let result = revert_from_history(&history_path, &RevertOptions::default());
        assert!(matches!(result, Err(RevertError::DirectoryNotFound(_))));
    }
    
    #[test]
    fn test_revert_creates_history() {
        let (dir, history_path) = setup_test_scenario();
        
        let options = RevertOptions { dry_run: false };
        revert_from_history(&history_path, &options).unwrap();
        
        // Check revert history was created
        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("-revert-"))
            .collect();
        
        assert_eq!(entries.len(), 1);
    }
    
    #[test]
    fn test_revert_conflict_detection() {
        let (dir, history_path) = setup_test_scenario();
        
        // Create conflicting directory (original name exists)
        fs::create_dir(dir.path().join("12345")).unwrap();
        
        let result = revert_from_history(&history_path, &RevertOptions::default());
        // Should fail because "12345" already exists
        assert!(matches!(result, Err(RevertError::DirectoryNotFound(_))));
    }
}
```

### Integration Tests

```rust
// tests/revert_integration_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_revert_cli() {
    // Test via CLI
}

#[test]
fn test_revert_invalid_history_file() {
    // Test error handling for invalid file
}
```

## Notes

- Revert operations also create history files for full auditability
- Double-revert is supported (revert of a revert)
- The tool validates all directories before making any changes
- Consider adding `--force` flag to skip missing directory validation
- Revert history filename includes both original and revert timestamps
