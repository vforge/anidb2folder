# 20 - Rename to Readable

## Summary

Convert directories from AniDB format to human-readable format using cached anime metadata.

## Dependencies

- **06-format-validator** — Requires validation to confirm all directories are in AniDB format
- **11-local-cache** — Requires cached API client for fetching anime metadata

## Description

This feature implements the core renaming logic to convert directories from the compact AniDB ID format to the human-readable format that includes anime titles, release years, and preserved series tags.

The rename process:

1. Validates all directories are in AniDB format (via feature 21)
2. For each directory, fetches anime metadata (via cached API)
3. Constructs the new directory name
4. Performs the filesystem rename operation
5. Records the change for history (via feature 41)

## Requirements

### Functional Requirements

1. Convert AniDB format directories to human-readable format
2. Preserve the series tag `[series]` if present
3. Construct the new name using:
   - Japanese title (romaji) — required
   - English title after `／` — if different from Japanese
   - Release year in parentheses — if available
   - AniDB ID suffix `[anidb-<id>]` — always
4. Handle missing metadata gracefully
5. Apply character sanitization (feature 30)
6. Apply name truncation if needed (feature 31)
7. Return a list of all changes made

### Non-Functional Requirements

1. Atomic rename operations (fs::rename)
2. Stop on first error, no partial renames
3. Clear logging of each rename operation

## Implementation Guide

### Step 1: Define Rename Types

```rust
// src/rename/types.rs
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct RenameOperation {
    pub source_path: PathBuf,
    pub source_name: String,
    pub destination_path: PathBuf,
    pub destination_name: String,
    pub anidb_id: u32,
    pub reason: String,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct RenameResult {
    pub operations: Vec<RenameOperation>,
    pub direction: RenameDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameDirection {
    AniDbToReadable,
    ReadableToAniDb,
}

#[derive(Error, Debug)]
pub enum RenameError {
    #[error("Failed to fetch anime data for ID {id}: {source}")]
    ApiError { id: u32, source: String },
    
    #[error("Failed to rename directory '{from}' to '{to}': {source}")]
    FilesystemError {
        from: String,
        to: String,
        source: std::io::Error,
    },
    
    #[error("Destination already exists: {0}")]
    DestinationExists(PathBuf),
    
    #[error("Name construction failed for ID {id}: {reason}")]
    NameConstructionError { id: u32, reason: String },
}
```

### Step 2: Implement Name Builder

```rust
// src/rename/name_builder.rs
use crate::api::AnimeInfo;
use crate::sanitizer::sanitize_filename;  // Feature 30
use crate::truncator::truncate_name;       // Feature 31

pub struct NameBuilderConfig {
    pub max_length: usize,
}

impl Default for NameBuilderConfig {
    fn default() -> Self {
        Self { max_length: 255 }
    }
}

pub struct NameBuildResult {
    pub name: String,
    pub truncated: bool,
}

/// Build a human-readable directory name from anime info
pub fn build_human_readable_name(
    series_tag: Option<&str>,
    info: &AnimeInfo,
    config: &NameBuilderConfig,
) -> NameBuildResult {
    let mut parts: Vec<String> = Vec::new();
    
    // Series tag
    if let Some(tag) = series_tag {
        parts.push(format!("[{}]", tag));
    }
    
    // Titles
    let title_part = build_title_part(&info.title_jp, info.title_en.as_deref());
    parts.push(title_part);
    
    // Year
    if let Some(year) = info.release_year {
        parts.push(format!("({})", year));
    }
    
    // AniDB ID suffix (always required)
    parts.push(format!("[anidb-{}]", info.anidb_id));
    
    // Join and sanitize
    let raw_name = parts.join(" ");
    let sanitized = sanitize_filename(&raw_name);
    
    // Truncate if needed
    if sanitized.len() > config.max_length {
        let truncated_name = truncate_name(
            series_tag,
            &info.title_jp,
            info.title_en.as_deref(),
            info.release_year,
            info.anidb_id,
            config.max_length,
        );
        
        NameBuildResult {
            name: truncated_name,
            truncated: true,
        }
    } else {
        NameBuildResult {
            name: sanitized,
            truncated: false,
        }
    }
}

fn build_title_part(title_jp: &str, title_en: Option<&str>) -> String {
    match title_en {
        Some(en) if en != title_jp => {
            // Use fullwidth slash as separator
            format!("{} ／ {}", title_jp, en)
        }
        _ => title_jp.to_string(),
    }
}
```

### Step 3: Implement Rename Executor

```rust
// src/rename/to_readable.rs
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::api::CachedAniDbClient;
use crate::parser::{AniDbFormat, ParsedDirectory};
use crate::validator::ValidationResult;

use super::name_builder::{build_human_readable_name, NameBuilderConfig, NameBuildResult};
use super::types::{RenameDirection, RenameError, RenameOperation, RenameResult};

pub struct RenameToReadableOptions {
    pub max_length: usize,
    pub dry_run: bool,
}

impl Default for RenameToReadableOptions {
    fn default() -> Self {
        Self {
            max_length: 255,
            dry_run: false,
        }
    }
}

/// Rename all directories from AniDB format to human-readable format
pub fn rename_to_readable(
    target_dir: &Path,
    validation: &ValidationResult,
    client: &mut CachedAniDbClient,
    options: &RenameToReadableOptions,
) -> Result<RenameResult, RenameError> {
    let config = NameBuilderConfig {
        max_length: options.max_length,
    };
    
    let mut operations = Vec::with_capacity(validation.directories.len());
    
    info!("Renaming {} directories to human-readable format", 
          validation.directories.len());
    
    for parsed in &validation.directories {
        let anidb_format = match parsed {
            ParsedDirectory::AniDb(f) => f,
            _ => continue, // Skip if somehow wrong format
        };
        
        let operation = prepare_rename_operation(
            target_dir,
            anidb_format,
            client,
            &config,
        )?;
        
        // Check destination doesn't already exist
        if operation.destination_path.exists() {
            return Err(RenameError::DestinationExists(
                operation.destination_path.clone()
            ));
        }
        
        operations.push(operation);
    }
    
    // Execute all renames (unless dry run)
    if !options.dry_run {
        for op in &operations {
            execute_rename(op)?;
        }
    }
    
    // Save cache after all operations
    if let Err(e) = client.save_cache() {
        warn!("Failed to save cache: {}", e);
    }
    
    Ok(RenameResult {
        operations,
        direction: RenameDirection::AniDbToReadable,
    })
}

fn prepare_rename_operation(
    target_dir: &Path,
    anidb: &AniDbFormat,
    client: &mut CachedAniDbClient,
    config: &NameBuilderConfig,
) -> Result<RenameOperation, RenameError> {
    debug!("Preparing rename for AniDB ID {}", anidb.anidb_id);
    
    // Fetch anime info
    let info = client.fetch_anime(anidb.anidb_id).map_err(|e| {
        RenameError::ApiError {
            id: anidb.anidb_id,
            source: e.to_string(),
        }
    })?;
    
    // Build new name
    let NameBuildResult { name, truncated } = build_human_readable_name(
        anidb.series_tag.as_deref(),
        &info,
        config,
    );
    
    if truncated {
        warn!(
            "Name truncated for AniDB ID {}: {} -> {}",
            anidb.anidb_id, info.title_jp, name
        );
    }
    
    let source_path = target_dir.join(&anidb.original_name);
    let destination_path = target_dir.join(&name);
    
    Ok(RenameOperation {
        source_path,
        source_name: anidb.original_name.clone(),
        destination_path,
        destination_name: name,
        anidb_id: anidb.anidb_id,
        reason: "Converted from AniDB format to human-readable format".to_string(),
        truncated,
    })
}

fn execute_rename(op: &RenameOperation) -> Result<(), RenameError> {
    info!("Renaming: {} -> {}", op.source_name, op.destination_name);
    
    fs::rename(&op.source_path, &op.destination_path).map_err(|e| {
        RenameError::FilesystemError {
            from: op.source_name.clone(),
            to: op.destination_name.clone(),
            source: e,
        }
    })
}
```

### Step 4: Integration

```rust
// src/rename/mod.rs
mod name_builder;
mod to_readable;
mod to_anidb;
mod types;

pub use to_readable::{rename_to_readable, RenameToReadableOptions};
pub use to_anidb::{rename_to_anidb, RenameToAniDbOptions};
pub use types::*;
```

```rust
// src/main.rs (updated for rename)
use crate::rename::{rename_to_readable, RenameToReadableOptions};
use crate::parser::DirectoryFormat;

fn main() -> Result<()> {
    // ... setup and validation ...
    
    match validation.format {
        DirectoryFormat::AniDb => {
            let options = RenameToReadableOptions {
                max_length: args.max_length,
                dry_run: args.dry,
            };
            
            let result = rename_to_readable(
                target_dir,
                &validation,
                &mut cached_client,
                &options,
            )?;
            
            info!("Renamed {} directories", result.operations.len());
            
            // TODO: Write history (feature 41)
        }
        DirectoryFormat::HumanReadable => {
            // Feature 23 handles this case
        }
    }
    
    Ok(())
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::AnimeInfo;
    
    #[test]
    fn test_build_name_full() {
        let info = AnimeInfo {
            anidb_id: 1,
            title_jp: "Cowboyu Bebopu".to_string(),
            title_en: Some("Cowboy Bebop".to_string()),
            release_year: Some(1998),
        };
        
        let result = build_human_readable_name(
            Some("AS0"),
            &info,
            &NameBuilderConfig::default(),
        );
        
        assert_eq!(
            result.name,
            "[AS0] Cowboyu Bebopu ／ Cowboy Bebop (1998) [anidb-1]"
        );
        assert!(!result.truncated);
    }
    
    #[test]
    fn test_build_name_no_series() {
        let info = AnimeInfo {
            anidb_id: 12345,
            title_jp: "Naruto".to_string(),
            title_en: None,
            release_year: Some(2002),
        };
        
        let result = build_human_readable_name(
            None,
            &info,
            &NameBuilderConfig::default(),
        );
        
        assert_eq!(result.name, "Naruto (2002) [anidb-12345]");
    }
    
    #[test]
    fn test_build_name_same_titles() {
        let info = AnimeInfo {
            anidb_id: 69,
            title_jp: "One Piece".to_string(),
            title_en: Some("One Piece".to_string()), // Same as JP
            release_year: Some(1999),
        };
        
        let result = build_human_readable_name(
            None,
            &info,
            &NameBuilderConfig::default(),
        );
        
        // Should not include duplicate title
        assert_eq!(result.name, "One Piece (1999) [anidb-69]");
    }
    
    #[test]
    fn test_build_name_no_year() {
        let info = AnimeInfo {
            anidb_id: 999,
            title_jp: "Unknown Anime".to_string(),
            title_en: None,
            release_year: None,
        };
        
        let result = build_human_readable_name(
            None,
            &info,
            &NameBuilderConfig::default(),
        );
        
        assert_eq!(result.name, "Unknown Anime [anidb-999]");
    }
    
    #[test]
    fn test_build_name_with_special_chars() {
        let info = AnimeInfo {
            anidb_id: 123,
            title_jp: "Title: With/Special*Chars?".to_string(),
            title_en: None,
            release_year: Some(2020),
        };
        
        let result = build_human_readable_name(
            None,
            &info,
            &NameBuilderConfig::default(),
        );
        
        // Special chars should be sanitized (feature 30)
        assert!(!result.name.contains('/'));
        assert!(!result.name.contains(':'));
        assert!(!result.name.contains('*'));
        assert!(!result.name.contains('?'));
    }
}
```

### Integration Tests

```rust
// tests/rename_to_readable_tests.rs
use anidb2folder::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_rename_to_readable_dry_run() {
    let dir = tempdir().unwrap();
    
    // Create AniDB format directories
    fs::create_dir(dir.path().join("[AS0] 1")).unwrap();
    
    let entries = scan_directory(dir.path()).unwrap();
    let validation = validate_directories(&entries).unwrap();
    
    // Mock API client needed here
    // let result = rename_to_readable(..., dry_run: true);
    
    // Verify original directories unchanged
    assert!(dir.path().join("[AS0] 1").exists());
}

#[test]
fn test_rename_to_readable_actual() {
    // Similar test but with dry_run: false
    // Verify directories are actually renamed
}

#[test]
fn test_rename_destination_conflict() {
    // Test that error is returned if destination exists
}
```

## Notes

- The rename operation is all-or-nothing — if any preparation fails, no renames are executed
- Cache is saved after successful rename to preserve fetched metadata
- Truncation warnings are logged but don't stop the operation
- Consider adding progress indicators for large directory sets
- The `dry_run` option prepares all operations but skips execution
