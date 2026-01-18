use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use chrono::Utc;
use tracing::{debug, error, info};

use crate::history::{
    read_history, HistoryDirection, HistoryEntry, HistoryError, HistoryFile, OperationType,
    HISTORY_VERSION,
};
use crate::progress::Progress;
use crate::rename::RenameDirection;

#[derive(Debug, thiserror::Error)]
pub enum RevertError {
    #[error("History error: {0}")]
    History(#[from] HistoryError),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Failed to rename '{from}' to '{to}': {source}")]
    RenameError {
        from: String,
        to: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write revert history: {0}")]
    WriteError(#[from] std::io::Error),

    #[error("Failed to serialize revert history: {0}")]
    SerializeError(#[from] serde_json::Error),
}

pub struct RevertOptions {
    pub dry_run: bool,
}

impl Default for RevertOptions {
    fn default() -> Self {
        Self { dry_run: false }
    }
}

/// A single revert operation
#[derive(Debug, Clone)]
pub struct RevertOperation {
    pub current_path: PathBuf,
    pub current_name: String,
    pub revert_path: PathBuf,
    pub revert_name: String,
    pub anidb_id: u32,
}

/// Result of a revert operation
#[derive(Debug)]
pub struct RevertResult {
    pub operations: Vec<RevertOperation>,
    /// TODO(feature-42): Display direction in revert UI output
    #[allow(dead_code)]
    pub direction: RenameDirection,
    pub original_history: PathBuf,
    pub dry_run: bool,
    pub revert_history_path: Option<PathBuf>,
}

/// Execute a revert operation using a history file
pub fn revert_from_history(
    history_path: &Path,
    options: &RevertOptions,
    progress: &mut Progress,
) -> Result<RevertResult, RevertError> {
    info!("Loading history from: {:?}", history_path);

    // Read history file
    let history = read_history(history_path)?;

    info!(
        "History contains {} changes from {}",
        history.changes.len(),
        history.executed_at
    );

    progress.revert_start(history.changes.len(), &history.executed_at.to_string());

    // Prepare revert operations
    let target_dir = &history.target_directory;
    let operations = prepare_revert_operations(&history, target_dir, progress)?;

    // Determine reversed direction
    let direction = match history.direction {
        HistoryDirection::AnidbToReadable => RenameDirection::ReadableToAniDb,
        HistoryDirection::ReadableToAnidb => RenameDirection::AniDbToReadable,
    };

    let mut revert_history_path = None;

    // Execute reverts (unless dry run)
    if !options.dry_run {
        execute_reverts(&operations, progress)?;

        // Write revert history
        let revert_time = Utc::now();
        let revert_history = create_revert_history(&history, &operations, &revert_time);
        let filename = history.generate_revert_filename(&revert_time);
        let revert_path = target_dir.join(&filename);

        write_revert_history(&revert_history, &revert_path)?;
        progress.history_written(&revert_path);

        info!("Revert history saved to: {:?}", revert_path);
        revert_history_path = Some(revert_path);
    }

    progress.revert_complete(operations.len(), options.dry_run);

    Ok(RevertResult {
        operations,
        direction,
        original_history: history_path.to_path_buf(),
        dry_run: options.dry_run,
        revert_history_path,
    })
}

fn prepare_revert_operations(
    history: &HistoryFile,
    target_dir: &Path,
    progress: &mut Progress,
) -> Result<Vec<RevertOperation>, RevertError> {
    let mut operations = Vec::with_capacity(history.changes.len());
    let mut errors = Vec::new();

    for entry in &history.changes {
        // For revert: source becomes destination, destination becomes source
        let current_path = target_dir.join(&entry.destination);
        let revert_path = target_dir.join(&entry.source);

        debug!(
            "Checking revert: {} -> {}",
            entry.destination, entry.source
        );

        // Check current (destination) exists
        if !current_path.exists() {
            errors.push(format!(
                "Directory not found: '{}' (expected from previous rename)",
                entry.destination
            ));
            continue;
        }

        // Check original (source) doesn't exist
        if revert_path.exists() {
            errors.push(format!(
                "Cannot revert: '{}' already exists",
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
            progress.warn(err);
        }
        return Err(RevertError::ValidationFailed(errors.join("; ")));
    }

    Ok(operations)
}

fn execute_reverts(
    operations: &[RevertOperation],
    progress: &mut Progress,
) -> Result<(), RevertError> {
    let total = operations.len();

    for (i, op) in operations.iter().enumerate() {
        progress.revert_progress(i + 1, total, &op.current_name, &op.revert_name);

        info!("Reverting: {} -> {}", op.current_name, op.revert_name);

        fs::rename(&op.current_path, &op.revert_path).map_err(|e| RevertError::RenameError {
            from: op.current_name.clone(),
            to: op.revert_name.clone(),
            source: e,
        })?;
    }

    Ok(())
}

fn create_revert_history(
    original: &HistoryFile,
    operations: &[RevertOperation],
    revert_time: &chrono::DateTime<Utc>,
) -> HistoryFile {
    let reversed_direction = match original.direction {
        HistoryDirection::AnidbToReadable => HistoryDirection::ReadableToAnidb,
        HistoryDirection::ReadableToAnidb => HistoryDirection::AnidbToReadable,
    };

    let changes: Vec<HistoryEntry> = operations
        .iter()
        .map(|op| HistoryEntry {
            source: op.current_name.clone(),
            destination: op.revert_name.clone(),
            anidb_id: op.anidb_id,
            truncated: false,
        })
        .collect();

    HistoryFile {
        version: HISTORY_VERSION.to_string(),
        executed_at: *revert_time,
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
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, history)?;
    }

    fs::rename(&temp_path, path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn test_progress() -> Progress {
        struct NullWriter;
        impl Write for NullWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        Progress::with_writer(Box::new(NullWriter))
    }

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
                    truncated: false,
                },
                HistoryEntry {
                    source: "[X] 99".to_string(),
                    destination: "[X] Other Title (2019) [anidb-99]".to_string(),
                    anidb_id: 99,
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
        let mut progress = test_progress();

        let options = RevertOptions { dry_run: false };
        let result = revert_from_history(&history_path, &options, &mut progress).unwrap();

        assert_eq!(result.operations.len(), 2);
        assert!(!result.dry_run);

        // Verify directories were reverted
        assert!(dir.path().join("12345").exists());
        assert!(dir.path().join("[X] 99").exists());

        // Verify original names are gone
        assert!(!dir
            .path()
            .join("Anime Title (2020) [anidb-12345]")
            .exists());
        assert!(!dir
            .path()
            .join("[X] Other Title (2019) [anidb-99]")
            .exists());
    }

    #[test]
    fn test_revert_dry_run() {
        let (dir, history_path) = setup_test_scenario();
        let mut progress = test_progress();

        let options = RevertOptions { dry_run: true };
        let result = revert_from_history(&history_path, &options, &mut progress).unwrap();

        assert_eq!(result.operations.len(), 2);
        assert!(result.dry_run);

        // Verify directories are NOT changed (dry run)
        assert!(dir
            .path()
            .join("Anime Title (2020) [anidb-12345]")
            .exists());
        assert!(!dir.path().join("12345").exists());
    }

    #[test]
    fn test_revert_missing_directory() {
        let dir = tempdir().unwrap();
        let mut progress = test_progress();

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
                truncated: false,
            }],
        };

        let history_path = dir.path().join("test-history.json");
        let file = fs::File::create(&history_path).unwrap();
        serde_json::to_writer_pretty(file, &history).unwrap();

        let result = revert_from_history(&history_path, &RevertOptions::default(), &mut progress);
        assert!(matches!(result, Err(RevertError::ValidationFailed(_))));
    }

    #[test]
    fn test_revert_creates_history() {
        let (_dir, history_path) = setup_test_scenario();
        let mut progress = test_progress();

        let options = RevertOptions { dry_run: false };
        let result = revert_from_history(&history_path, &options, &mut progress).unwrap();

        // Check revert history was created
        assert!(result.revert_history_path.is_some());
        assert!(result.revert_history_path.unwrap().exists());
    }

    #[test]
    fn test_revert_conflict_detection() {
        let (dir, history_path) = setup_test_scenario();
        let mut progress = test_progress();

        // Create conflicting directory (original name exists)
        fs::create_dir(dir.path().join("12345")).unwrap();

        let result = revert_from_history(&history_path, &RevertOptions::default(), &mut progress);
        // Should fail because "12345" already exists
        assert!(matches!(result, Err(RevertError::ValidationFailed(_))));
    }

    #[test]
    fn test_revert_direction_reversed() {
        let (_dir, history_path) = setup_test_scenario();
        let mut progress = test_progress();

        let options = RevertOptions { dry_run: true };
        let result = revert_from_history(&history_path, &options, &mut progress).unwrap();

        // Original was AnidbToReadable, so revert should be ReadableToAniDb
        assert_eq!(result.direction, RenameDirection::ReadableToAniDb);
    }
}
