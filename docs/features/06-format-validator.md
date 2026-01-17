# 06 - Format Validator

## Summary

Validate that all subdirectories in a target directory are in a consistent format before any renaming operations.

## Dependencies

- **05-format-parser** — Requires parsing functions to identify directory formats

## Description

This feature implements validation logic that ensures all subdirectories in the target directory are in the same format (either all AniDB format or all human-readable format). If any directories are unrecognized or if formats are mixed, the tool must exit immediately with clear error messages.

This validation is a critical safety feature that prevents partial renames and data corruption.

## Requirements

### Functional Requirements

1. Scan all subdirectories and parse each one
2. Determine if all directories are in the same format
3. Return validation results indicating:
   - Success: all directories match one format (specify which)
   - Failure: unrecognized directories (list them)
   - Failure: mixed formats (list directories by format)
4. Provide detailed error output for debugging

### Non-Functional Requirements

1. Fail fast — exit on first validation error
2. Comprehensive error messages
3. Include all problematic directories in error output (not just the first)

## Implementation Guide

### Step 1: Define Validation Types

```rust
// src/validator/types.rs
use std::path::PathBuf;
use thiserror::Error;

use crate::parser::{DirectoryFormat, ParsedDirectory};

#[derive(Debug)]
pub struct ValidationResult {
    pub format: DirectoryFormat,
    pub directories: Vec<ParsedDirectory>,
}

#[derive(Debug)]
pub struct FormatMismatch {
    pub anidb_dirs: Vec<String>,
    pub human_readable_dirs: Vec<String>,
}

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Unrecognized directory format")]
    UnrecognizedDirectories {
        directories: Vec<String>,
    },
    
    #[error("Mixed directory formats found")]
    MixedFormats {
        mismatch: FormatMismatch,
    },
    
    #[error("No directories found in target")]
    NoDirectories,
}

impl ValidationError {
    pub fn format_error_message(&self) -> String {
        match self {
            ValidationError::UnrecognizedDirectories { directories } => {
                let mut msg = String::from(
                    "The following directories do not match any known format:\n"
                );
                for dir in directories {
                    msg.push_str(&format!("  - {}\n", dir));
                }
                msg.push_str("\nExpected formats:\n");
                msg.push_str("  AniDB:          [<series>] <anidb_id>\n");
                msg.push_str("  Human-readable: [<series>] <title> (<year>) [anidb-<id>]\n");
                msg
            }
            ValidationError::MixedFormats { mismatch } => {
                let mut msg = String::from(
                    "Found directories in multiple formats. All directories must be in the same format.\n\n"
                );
                
                if !mismatch.anidb_dirs.is_empty() {
                    msg.push_str("AniDB format directories:\n");
                    for dir in &mismatch.anidb_dirs {
                        msg.push_str(&format!("  - {}\n", dir));
                    }
                    msg.push('\n');
                }
                
                if !mismatch.human_readable_dirs.is_empty() {
                    msg.push_str("Human-readable format directories:\n");
                    for dir in &mismatch.human_readable_dirs {
                        msg.push_str(&format!("  - {}\n", dir));
                    }
                }
                
                msg
            }
            ValidationError::NoDirectories => {
                String::from("No subdirectories found in the target directory.")
            }
        }
    }
}
```

### Step 2: Implement Validator

```rust
// src/validator/mod.rs
mod types;

pub use types::*;

use crate::parser::{parse_directory_name, DirectoryFormat, ParsedDirectory};
use crate::scanner::DirectoryEntry;
use tracing::{debug, error, info};

/// Validate that all directories are in the same format
pub fn validate_directories(
    entries: &[DirectoryEntry]
) -> Result<ValidationResult, ValidationError> {
    if entries.is_empty() {
        return Err(ValidationError::NoDirectories);
    }
    
    info!("Validating {} directories", entries.len());
    
    let mut parsed: Vec<ParsedDirectory> = Vec::with_capacity(entries.len());
    let mut unrecognized: Vec<String> = Vec::new();
    let mut anidb_dirs: Vec<String> = Vec::new();
    let mut human_readable_dirs: Vec<String> = Vec::new();
    
    // Parse all directories
    for entry in entries {
        match parse_directory_name(&entry.name) {
            Ok(p) => {
                debug!("Parsed '{}' as {:?}", entry.name, p.format());
                
                match p.format() {
                    DirectoryFormat::AniDb => anidb_dirs.push(entry.name.clone()),
                    DirectoryFormat::HumanReadable => human_readable_dirs.push(entry.name.clone()),
                }
                
                parsed.push(p);
            }
            Err(_) => {
                debug!("Unrecognized format: '{}'", entry.name);
                unrecognized.push(entry.name.clone());
            }
        }
    }
    
    // Check for unrecognized directories
    if !unrecognized.is_empty() {
        error!("{} directories have unrecognized format", unrecognized.len());
        return Err(ValidationError::UnrecognizedDirectories {
            directories: unrecognized,
        });
    }
    
    // Check for mixed formats
    let has_anidb = !anidb_dirs.is_empty();
    let has_human_readable = !human_readable_dirs.is_empty();
    
    if has_anidb && has_human_readable {
        error!(
            "Mixed formats: {} AniDB, {} human-readable",
            anidb_dirs.len(),
            human_readable_dirs.len()
        );
        return Err(ValidationError::MixedFormats {
            mismatch: FormatMismatch {
                anidb_dirs,
                human_readable_dirs,
            },
        });
    }
    
    // Determine the format
    let format = if has_anidb {
        DirectoryFormat::AniDb
    } else {
        DirectoryFormat::HumanReadable
    };
    
    info!("Validation passed: all {} directories are in {:?} format", 
          parsed.len(), format);
    
    Ok(ValidationResult {
        format,
        directories: parsed,
    })
}

/// Quick validation without full parsing results
pub fn quick_validate(entries: &[DirectoryEntry]) -> Result<DirectoryFormat, ValidationError> {
    validate_directories(entries).map(|r| r.format)
}
```

### Step 3: Integrate with Main

```rust
// src/main.rs (updated)
use crate::validator::{validate_directories, ValidationError};

fn main() -> Result<()> {
    let args = Args::parse();
    // ... setup ...
    
    if let Some(target_dir) = &args.target_dir {
        let entries = scan_directory(target_dir)?;
        
        // Validate all directories
        let validation = match validate_directories(&entries) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}", e.format_error_message());
                std::process::exit(match &e {
                    ValidationError::UnrecognizedDirectories { .. } => 5,
                    ValidationError::MixedFormats { .. } => 4,
                    ValidationError::NoDirectories => 3,
                });
            }
        };
        
        info!("All directories are in {:?} format", validation.format);
        
        // Proceed with renaming...
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
    use crate::scanner::DirectoryEntry;
    use std::path::PathBuf;
    
    fn make_entry(name: &str) -> DirectoryEntry {
        DirectoryEntry {
            name: name.to_string(),
            path: PathBuf::from(format!("/test/{}", name)),
        }
    }
    
    #[test]
    fn test_validate_all_anidb() {
        let entries = vec![
            make_entry("12345"),
            make_entry("[AS0] 67890"),
            make_entry("[Series] 11111"),
        ];
        
        let result = validate_directories(&entries).unwrap();
        
        assert_eq!(result.format, DirectoryFormat::AniDb);
        assert_eq!(result.directories.len(), 3);
    }
    
    #[test]
    fn test_validate_all_human_readable() {
        let entries = vec![
            make_entry("Naruto (2002) [anidb-12345]"),
            make_entry("[AS0] Cowboy Bebop (1998) [anidb-1]"),
            make_entry("One Piece [anidb-69]"),
        ];
        
        let result = validate_directories(&entries).unwrap();
        
        assert_eq!(result.format, DirectoryFormat::HumanReadable);
        assert_eq!(result.directories.len(), 3);
    }
    
    #[test]
    fn test_validate_mixed_formats_error() {
        let entries = vec![
            make_entry("12345"),                        // AniDB
            make_entry("Naruto (2002) [anidb-67890]"),  // Human-readable
        ];
        
        let result = validate_directories(&entries);
        
        assert!(matches!(result, Err(ValidationError::MixedFormats { .. })));
        
        if let Err(ValidationError::MixedFormats { mismatch }) = result {
            assert_eq!(mismatch.anidb_dirs.len(), 1);
            assert_eq!(mismatch.human_readable_dirs.len(), 1);
        }
    }
    
    #[test]
    fn test_validate_unrecognized_error() {
        let entries = vec![
            make_entry("12345"),
            make_entry("Random Folder"),  // Invalid
            make_entry("Another Invalid"),
        ];
        
        let result = validate_directories(&entries);
        
        assert!(matches!(result, Err(ValidationError::UnrecognizedDirectories { .. })));
        
        if let Err(ValidationError::UnrecognizedDirectories { directories }) = result {
            assert_eq!(directories.len(), 2);
            assert!(directories.contains(&"Random Folder".to_string()));
            assert!(directories.contains(&"Another Invalid".to_string()));
        }
    }
    
    #[test]
    fn test_validate_empty_error() {
        let entries: Vec<DirectoryEntry> = vec![];
        
        let result = validate_directories(&entries);
        
        assert!(matches!(result, Err(ValidationError::NoDirectories)));
    }
    
    #[test]
    fn test_validate_single_directory() {
        let entries = vec![make_entry("[X] 99999")];
        
        let result = validate_directories(&entries).unwrap();
        
        assert_eq!(result.format, DirectoryFormat::AniDb);
        assert_eq!(result.directories.len(), 1);
    }
    
    #[test]
    fn test_error_message_unrecognized() {
        let err = ValidationError::UnrecognizedDirectories {
            directories: vec!["Invalid1".to_string(), "Invalid2".to_string()],
        };
        
        let msg = err.format_error_message();
        
        assert!(msg.contains("Invalid1"));
        assert!(msg.contains("Invalid2"));
        assert!(msg.contains("Expected formats:"));
    }
    
    #[test]
    fn test_error_message_mixed() {
        let err = ValidationError::MixedFormats {
            mismatch: FormatMismatch {
                anidb_dirs: vec!["12345".to_string()],
                human_readable_dirs: vec!["Title [anidb-1]".to_string()],
            },
        };
        
        let msg = err.format_error_message();
        
        assert!(msg.contains("12345"));
        assert!(msg.contains("Title [anidb-1]"));
        assert!(msg.contains("AniDB format"));
        assert!(msg.contains("Human-readable"));
    }
}
```

### Integration Tests

```rust
// tests/validation_integration_tests.rs
use anidb2folder::{scan_directory, validate_directories};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_validation_with_real_directories() {
    let dir = tempdir().unwrap();
    
    fs::create_dir(dir.path().join("12345")).unwrap();
    fs::create_dir(dir.path().join("[AS0] 67890")).unwrap();
    
    let entries = scan_directory(dir.path()).unwrap();
    let result = validate_directories(&entries).unwrap();
    
    assert_eq!(result.directories.len(), 2);
}

#[test]
fn test_exit_code_on_mixed_formats() {
    // Test via CLI that correct exit code is returned
}
```

## Notes

- Validation happens **before** any renaming operations
- All problematic directories are listed, not just the first one found
- Error messages are designed to be user-friendly and actionable
- Exit codes are documented in feature 51 (error handling)
- Consider adding a `--force` flag in the future to allow mixed format handling
