use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::api::{AniDbClient, AnimeInfo, ApiConfig, ApiError};
use crate::cache::{CacheConfig, CacheStore};
use crate::parser::{AniDbFormat, ParsedDirectory};
use crate::progress::Progress;
use crate::validator::ValidationResult;

use super::name_builder::{build_human_readable_name, NameBuildResult, NameBuilderConfig};
use super::types::{RenameDirection, RenameOperation, RenameResult};

/// Errors that can occur during rename operations
#[derive(Error, Debug)]
pub enum RenameError {
    #[error("Failed to fetch anime data for ID {id}: {message}")]
    ApiError { id: u32, message: String },

    #[error("Failed to rename '{from}' to '{to}': {source}")]
    FilesystemError {
        from: String,
        to: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Destination already exists: {0}")]
    DestinationExists(String),

    #[error("API client not configured")]
    ApiNotConfigured,

    #[error("Cache error: {0}")]
    CacheError(String),
}

impl From<ApiError> for RenameError {
    fn from(err: ApiError) -> Self {
        RenameError::ApiError {
            id: 0,
            message: err.to_string(),
        }
    }
}

/// Options for rename to readable operation
#[derive(Debug, Clone)]
pub struct RenameOptions {
    pub max_length: usize,
    pub dry_run: bool,
    pub cache_expiry_days: u32,
}

impl Default for RenameOptions {
    fn default() -> Self {
        Self {
            max_length: 255,
            dry_run: false,
            cache_expiry_days: 30,
        }
    }
}

/// Rename directories from AniDB format to human-readable format
pub fn rename_to_readable(
    target_dir: &Path,
    validation: &ValidationResult,
    api_config: &ApiConfig,
    options: &RenameOptions,
    progress: &mut Progress,
) -> Result<RenameResult, RenameError> {
    // Setup cache
    let cache_config = CacheConfig::for_target_dir(target_dir, options.cache_expiry_days);
    let mut cache = CacheStore::load(cache_config);

    // Setup API client (only if we need to fetch)
    let api_client = if api_config.is_configured() {
        Some(
            AniDbClient::new(api_config.clone())
                .map_err(|e| RenameError::ApiError {
                    id: 0,
                    message: e.to_string(),
                })?,
        )
    } else {
        None
    };

    let name_config = NameBuilderConfig {
        max_length: options.max_length,
    };

    let mut result = RenameResult::new(RenameDirection::AniDbToReadable, options.dry_run);
    let total = validation.directories.len();

    info!(
        "Preparing to rename {} directories to human-readable format",
        total
    );

    // First pass: prepare all operations (fetch data, build names)
    for (i, parsed) in validation.directories.iter().enumerate() {
        let anidb_format = match parsed {
            ParsedDirectory::AniDb(f) => f,
            _ => continue, // Skip if somehow wrong format
        };

        let operation = prepare_rename_operation(
            target_dir,
            anidb_format,
            &mut cache,
            api_client.as_ref(),
            &name_config,
            progress,
            options.dry_run,
        )?;

        // Check destination doesn't already exist
        if operation.destination_path.exists() && !options.dry_run {
            return Err(RenameError::DestinationExists(
                operation.destination_name.clone(),
            ));
        }

        progress.rename_progress(
            i + 1,
            total,
            &operation.source_name,
            &operation.destination_name,
        );

        result.add_operation(operation);
    }

    // Second pass: execute all renames (unless dry run)
    if !options.dry_run {
        for op in &result.operations {
            execute_rename(op)?;
        }

        info!("Successfully renamed {} directories", result.len());
    }

    // Save cache
    if let Err(e) = cache.save() {
        warn!("Failed to save cache: {}", e);
    }

    Ok(result)
}

fn prepare_rename_operation(
    target_dir: &Path,
    anidb: &AniDbFormat,
    cache: &mut CacheStore,
    api_client: Option<&AniDbClient>,
    config: &NameBuilderConfig,
    progress: &mut Progress,
    dry_run: bool,
) -> Result<RenameOperation, RenameError> {
    debug!("Preparing rename for AniDB ID {}", anidb.anidb_id);

    // Try cache first
    let info = if let Some(cached) = cache.get(anidb.anidb_id) {
        debug!("Using cached data for AniDB ID {}", anidb.anidb_id);
        progress.using_cache(anidb.anidb_id);
        cached
    } else if dry_run {
        // In dry run mode, don't call API - use placeholder data
        debug!("Dry run: using placeholder for AniDB ID {}", anidb.anidb_id);
        progress.would_fetch(anidb.anidb_id);
        AnimeInfo {
            anidb_id: anidb.anidb_id,
            title_main: format!("[Title for anidb-{}]", anidb.anidb_id),
            title_en: None,
            release_year: None,
        }
    } else {
        // Fetch from API
        let client = api_client.ok_or(RenameError::ApiNotConfigured)?;

        info!("Fetching data for AniDB ID {} from API", anidb.anidb_id);
        progress.fetch_start(anidb.anidb_id);
        let info = client.fetch_anime(anidb.anidb_id).map_err(|e| {
            RenameError::ApiError {
                id: anidb.anidb_id,
                message: e.to_string(),
            }
        })?;
        progress.fetch_complete();

        // Cache the result
        cache.insert(&info);
        info
    };

    // Build new name
    let NameBuildResult { name, truncated } =
        build_human_readable_name(anidb.series_tag.as_deref(), &info, config);

    if truncated {
        warn!(
            "Name truncated for AniDB ID {}: {} -> {}",
            anidb.anidb_id, info.title_main, name
        );
        progress.warn(&format!(
            "Name truncated for {}: {}",
            anidb.anidb_id, info.title_main
        ));
    }

    let source_path = target_dir.join(&anidb.original_name);

    Ok(RenameOperation::new(source_path, name, anidb.anidb_id, truncated))
}

fn execute_rename(op: &RenameOperation) -> Result<(), RenameError> {
    info!("Renaming: {} -> {}", op.source_name, op.destination_name);

    fs::rename(&op.source_path, &op.destination_path).map_err(|e| RenameError::FilesystemError {
        from: op.source_name.clone(),
        to: op.destination_name.clone(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::AnimeInfo;
    use crate::scanner::DirectoryEntry;
    use crate::validator::validate_directories;
    use std::io::Write;
    use tempfile::tempdir;

    fn make_entry(name: &str, path: &Path) -> DirectoryEntry {
        DirectoryEntry {
            name: name.to_string(),
            path: path.join(name),
        }
    }

    /// Create a test progress reporter that writes to a buffer
    fn test_progress() -> Progress {
        // Use a null writer for tests
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

    #[test]
    fn test_rename_options_default() {
        let opts = RenameOptions::default();
        assert_eq!(opts.max_length, 255);
        assert!(!opts.dry_run);
        assert_eq!(opts.cache_expiry_days, 30);
    }

    #[test]
    fn test_prepare_rename_requires_api_when_not_cached() {
        let dir = tempdir().unwrap();
        let cache_config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(cache_config);
        let config = NameBuilderConfig::default();
        let mut progress = test_progress();

        let anidb = AniDbFormat {
            series_tag: None,
            anidb_id: 12345,
            original_name: "12345".to_string(),
        };

        // Without API client and not in dry run mode, should fail
        let result =
            prepare_rename_operation(dir.path(), &anidb, &mut cache, None, &config, &mut progress, false);

        assert!(matches!(result, Err(RenameError::ApiNotConfigured)));
    }

    #[test]
    fn test_prepare_rename_dry_run_uses_placeholder() {
        let dir = tempdir().unwrap();
        let cache_config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(cache_config);
        let config = NameBuilderConfig::default();
        let mut progress = test_progress();

        let anidb = AniDbFormat {
            series_tag: None,
            anidb_id: 12345,
            original_name: "12345".to_string(),
        };

        // In dry run mode without cache, should use placeholder
        let result =
            prepare_rename_operation(dir.path(), &anidb, &mut cache, None, &config, &mut progress, true);

        assert!(result.is_ok());
        let op = result.unwrap();
        assert!(op.destination_name.contains("[Title for anidb-12345]"));
    }

    #[test]
    fn test_prepare_rename_uses_cache() {
        let dir = tempdir().unwrap();
        let cache_config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(cache_config);
        let config = NameBuilderConfig::default();
        let mut progress = test_progress();

        // Pre-populate cache
        let info = AnimeInfo {
            anidb_id: 12345,
            title_main: "Test Anime".to_string(),
            title_en: Some("Test Anime EN".to_string()),
            release_year: Some(2020),
        };
        cache.insert(&info);

        let anidb = AniDbFormat {
            series_tag: Some("X".to_string()),
            anidb_id: 12345,
            original_name: "[X] 12345".to_string(),
        };

        // Should succeed using cache (no API client needed)
        let result =
            prepare_rename_operation(dir.path(), &anidb, &mut cache, None, &config, &mut progress, false);

        assert!(result.is_ok());
        let op = result.unwrap();
        assert_eq!(op.anidb_id, 12345);
        assert!(op.destination_name.contains("Test Anime"));
        assert!(op.destination_name.contains("[X]"));
        assert!(op.destination_name.contains("[anidb-12345]"));
    }

    #[test]
    fn test_rename_dry_run_no_filesystem_changes() {
        let dir = tempdir().unwrap();
        let mut progress = test_progress();

        // Create test directories
        std::fs::create_dir(dir.path().join("12345")).unwrap();

        // Pre-populate cache so we don't need API
        let cache_config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(cache_config);
        cache.insert(&AnimeInfo {
            anidb_id: 12345,
            title_main: "Test Anime".to_string(),
            title_en: None,
            release_year: Some(2020),
        });
        cache.save().unwrap();

        let entries = vec![make_entry("12345", dir.path())];
        let validation = validate_directories(&entries).unwrap();

        let options = RenameOptions {
            dry_run: true,
            ..Default::default()
        };

        let result = rename_to_readable(
            dir.path(),
            &validation,
            &ApiConfig::default(),
            &options,
            &mut progress,
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.dry_run);

        // Original directory should still exist
        assert!(dir.path().join("12345").exists());
    }

    #[test]
    fn test_rename_actual_execution() {
        let dir = tempdir().unwrap();
        let mut progress = test_progress();

        // Create test directory
        std::fs::create_dir(dir.path().join("12345")).unwrap();

        // Pre-populate cache
        let cache_config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(cache_config);
        cache.insert(&AnimeInfo {
            anidb_id: 12345,
            title_main: "Test Anime".to_string(),
            title_en: None,
            release_year: Some(2020),
        });
        cache.save().unwrap();

        let entries = vec![make_entry("12345", dir.path())];
        let validation = validate_directories(&entries).unwrap();

        let options = RenameOptions {
            dry_run: false,
            ..Default::default()
        };

        let result = rename_to_readable(
            dir.path(),
            &validation,
            &ApiConfig::default(),
            &options,
            &mut progress,
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.len(), 1);
        assert!(!result.dry_run);

        // Original directory should NOT exist
        assert!(!dir.path().join("12345").exists());

        // New directory should exist
        assert!(dir.path().join("Test Anime (2020) [anidb-12345]").exists());
    }

    #[test]
    fn test_rename_preserves_series_tag() {
        let dir = tempdir().unwrap();
        let mut progress = test_progress();

        // Create test directory with series tag
        std::fs::create_dir(dir.path().join("[AS0] 12345")).unwrap();

        // Pre-populate cache
        let cache_config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(cache_config);
        cache.insert(&AnimeInfo {
            anidb_id: 12345,
            title_main: "Test Anime".to_string(),
            title_en: None,
            release_year: Some(2020),
        });
        cache.save().unwrap();

        let entries = vec![make_entry("[AS0] 12345", dir.path())];
        let validation = validate_directories(&entries).unwrap();

        let options = RenameOptions {
            dry_run: false,
            ..Default::default()
        };

        let result = rename_to_readable(
            dir.path(),
            &validation,
            &ApiConfig::default(),
            &options,
            &mut progress,
        );

        assert!(result.is_ok());

        // New directory should have series tag
        assert!(dir
            .path()
            .join("[AS0] Test Anime (2020) [anidb-12345]")
            .exists());
    }

    #[test]
    fn test_rename_error_destination_exists() {
        let dir = tempdir().unwrap();
        let mut progress = test_progress();

        // Create source and destination directories
        std::fs::create_dir(dir.path().join("12345")).unwrap();
        std::fs::create_dir(dir.path().join("Test Anime (2020) [anidb-12345]")).unwrap();

        // Pre-populate cache
        let cache_config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(cache_config);
        cache.insert(&AnimeInfo {
            anidb_id: 12345,
            title_main: "Test Anime".to_string(),
            title_en: None,
            release_year: Some(2020),
        });
        cache.save().unwrap();

        let entries = vec![make_entry("12345", dir.path())];
        let validation = validate_directories(&entries).unwrap();

        let options = RenameOptions {
            dry_run: false,
            ..Default::default()
        };

        let result = rename_to_readable(
            dir.path(),
            &validation,
            &ApiConfig::default(),
            &options,
            &mut progress,
        );

        assert!(matches!(result, Err(RenameError::DestinationExists(_))));
    }
}
