# 21 - Rename to AniDB

## Summary

Convert directories from human-readable format back to the compact AniDB ID format.

## Dependencies

- **06-format-validator** — Requires validation to confirm all directories are in human-readable format

## Description

This feature implements the reverse renaming operation, converting directories from the human-readable format back to the compact AniDB ID format. This is the complementary operation to feature 22.

The rename process:

1. Validates all directories are in human-readable format (via feature 21)
2. For each directory, extracts the AniDB ID and optional series tag
3. Constructs the compact directory name
4. Performs the filesystem rename operation
5. Records the change for history (via feature 41)

Unlike renaming to human-readable format, this operation does **not** require API calls since all necessary information (series tag and AniDB ID) is already embedded in the directory name.

## Requirements

### Functional Requirements

1. Convert human-readable format directories to AniDB format
2. Preserve the series tag `[series]` if present
3. Construct the new name using:
   - Optional series tag: `[series]` — if present in source
   - AniDB ID — extracted from `[anidb-<id>]` suffix
4. Return a list of all changes made

### Non-Functional Requirements

1. Atomic rename operations (fs::rename)
2. Stop on first error, no partial renames
3. No API calls required

## Implementation Guide

### Step 1: Implement Name Builder for AniDB Format

```rust
// src/rename/name_builder.rs (additions)

/// Build an AniDB format directory name from parsed data
pub fn build_anidb_name(series_tag: Option<&str>, anidb_id: u32) -> String {
    match series_tag {
        Some(tag) => format!("[{}] {}", tag, anidb_id),
        None => anidb_id.to_string(),
    }
}
```

### Step 2: Implement Rename Executor

```rust
// src/rename/to_anidb.rs
use std::fs;
use std::path::Path;
use tracing::{debug, info};

use crate::parser::{HumanReadableFormat, ParsedDirectory};
use crate::validator::ValidationResult;

use super::name_builder::build_anidb_name;
use super::types::{RenameDirection, RenameError, RenameOperation, RenameResult};

pub struct RenameToAniDbOptions {
    pub dry_run: bool,
}

impl Default for RenameToAniDbOptions {
    fn default() -> Self {
        Self { dry_run: false }
    }
}

/// Rename all directories from human-readable format to AniDB format
pub fn rename_to_anidb(
    target_dir: &Path,
    validation: &ValidationResult,
    options: &RenameToAniDbOptions,
) -> Result<RenameResult, RenameError> {
    let mut operations = Vec::with_capacity(validation.directories.len());
    
    info!("Renaming {} directories to AniDB format", 
          validation.directories.len());
    
    for parsed in &validation.directories {
        let hr_format = match parsed {
            ParsedDirectory::HumanReadable(f) => f,
            _ => continue, // Skip if somehow wrong format
        };
        
        let operation = prepare_rename_operation(target_dir, hr_format)?;
        
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
    
    Ok(RenameResult {
        operations,
        direction: RenameDirection::ReadableToAniDb,
    })
}

fn prepare_rename_operation(
    target_dir: &Path,
    hr: &HumanReadableFormat,
) -> Result<RenameOperation, RenameError> {
    debug!("Preparing rename for '{}'", hr.original_name);
    
    // Build new name (simple - just series tag + ID)
    let new_name = build_anidb_name(hr.series_tag.as_deref(), hr.anidb_id);
    
    let source_path = target_dir.join(&hr.original_name);
    let destination_path = target_dir.join(&new_name);
    
    Ok(RenameOperation {
        source_path,
        source_name: hr.original_name.clone(),
        destination_path,
        destination_name: new_name,
        anidb_id: hr.anidb_id,
        reason: "Converted from human-readable format to AniDB format".to_string(),
        truncated: false, // Never truncated for AniDB format
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

### Step 3: Update Main Integration

```rust
// src/main.rs (updated for both rename directions)
use crate::rename::{
    rename_to_readable, RenameToReadableOptions,
    rename_to_anidb, RenameToAniDbOptions,
};
use crate::parser::DirectoryFormat;

fn main() -> Result<()> {
    // ... setup and validation ...
    
    let result = match validation.format {
        DirectoryFormat::AniDb => {
            let options = RenameToReadableOptions {
                max_length: args.max_length,
                dry_run: args.dry,
            };
            
            rename_to_readable(
                target_dir,
                &validation,
                &mut cached_client,
                &options,
            )?
        }
        DirectoryFormat::HumanReadable => {
            let options = RenameToAniDbOptions {
                dry_run: args.dry,
            };
            
            rename_to_anidb(
                target_dir,
                &validation,
                &options,
            )?
        }
    };
    
    info!("Renamed {} directories", result.operations.len());
    
    // TODO: Write history (feature 41)
    
    Ok(())
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_build_anidb_name_with_series() {
        let name = build_anidb_name(Some("AS0"), 12345);
        assert_eq!(name, "[AS0] 12345");
    }
    
    #[test]
    fn test_build_anidb_name_no_series() {
        let name = build_anidb_name(None, 67890);
        assert_eq!(name, "67890");
    }
    
    #[test]
    fn test_build_anidb_name_complex_series() {
        let name = build_anidb_name(Some("My Favorite Series"), 111);
        assert_eq!(name, "[My Favorite Series] 111");
    }
}
```

### Integration Tests

```rust
// tests/rename_to_anidb_tests.rs
use anidb2folder::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_rename_to_anidb_full() {
    let dir = tempdir().unwrap();
    
    // Create human-readable directories
    fs::create_dir(dir.path().join("[AS0] Cowboyu Bebopu ／ Cowboy Bebop (1998) [anidb-1]")).unwrap();
    fs::create_dir(dir.path().join("Naruto (2002) [anidb-12345]")).unwrap();
    
    let entries = scan_directory(dir.path()).unwrap();
    let validation = validate_directories(&entries).unwrap();
    
    let options = RenameToAniDbOptions { dry_run: false };
    let result = rename_to_anidb(dir.path(), &validation, &options).unwrap();
    
    assert_eq!(result.operations.len(), 2);
    
    // Verify renames
    assert!(dir.path().join("[AS0] 1").exists());
    assert!(dir.path().join("12345").exists());
    
    // Verify originals are gone
    assert!(!dir.path().join("[AS0] Cowboyu Bebopu ／ Cowboy Bebop (1998) [anidb-1]").exists());
    assert!(!dir.path().join("Naruto (2002) [anidb-12345]").exists());
}

#[test]
fn test_rename_to_anidb_dry_run() {
    let dir = tempdir().unwrap();
    
    let original_name = "[Series] Title (2020) [anidb-999]";
    fs::create_dir(dir.path().join(original_name)).unwrap();
    
    let entries = scan_directory(dir.path()).unwrap();
    let validation = validate_directories(&entries).unwrap();
    
    let options = RenameToAniDbOptions { dry_run: true };
    let result = rename_to_anidb(dir.path(), &validation, &options).unwrap();
    
    assert_eq!(result.operations.len(), 1);
    assert_eq!(result.operations[0].destination_name, "[Series] 999");
    
    // Verify original still exists (dry run)
    assert!(dir.path().join(original_name).exists());
    assert!(!dir.path().join("[Series] 999").exists());
}

#[test]
fn test_rename_to_anidb_preserves_series_tag() {
    let dir = tempdir().unwrap();
    
    fs::create_dir(dir.path().join("[MyTag] Some Title (2015) [anidb-777]")).unwrap();
    
    let entries = scan_directory(dir.path()).unwrap();
    let validation = validate_directories(&entries).unwrap();
    
    let options = RenameToAniDbOptions { dry_run: false };
    let result = rename_to_anidb(dir.path(), &validation, &options).unwrap();
    
    // Series tag should be preserved
    assert!(dir.path().join("[MyTag] 777").exists());
}

#[test]
fn test_rename_to_anidb_no_series_tag() {
    let dir = tempdir().unwrap();
    
    fs::create_dir(dir.path().join("Title Without Tag (2018) [anidb-555]")).unwrap();
    
    let entries = scan_directory(dir.path()).unwrap();
    let validation = validate_directories(&entries).unwrap();
    
    let options = RenameToAniDbOptions { dry_run: false };
    let result = rename_to_anidb(dir.path(), &validation, &options).unwrap();
    
    // Should just be the ID
    assert!(dir.path().join("555").exists());
}

#[test]
fn test_rename_destination_conflict() {
    let dir = tempdir().unwrap();
    
    // Create human-readable that would become "12345"
    fs::create_dir(dir.path().join("Title (2020) [anidb-12345]")).unwrap();
    
    // Create conflicting destination
    fs::create_dir(dir.path().join("12345")).unwrap();
    
    let entries = scan_directory(dir.path()).unwrap();
    // Note: This would fail validation due to mixed formats
    // For a proper test, would need to construct validation manually
}
```

### Roundtrip Test

```rust
#[test]
fn test_roundtrip_rename() {
    let dir = tempdir().unwrap();
    
    // Start with AniDB format
    fs::create_dir(dir.path().join("[X] 12345")).unwrap();
    
    // This test would require mocking the API client
    // 1. Rename to readable format
    // 2. Rename back to AniDB format
    // 3. Verify we get back "[X] 12345"
}
```

## Notes

- This operation is much simpler than renaming to readable format since no API calls are needed
- All information needed is already in the directory name
- Series tags are always preserved exactly as they were
- No truncation is ever needed since AniDB format names are short
- The operation is fast since it's purely filesystem operations
