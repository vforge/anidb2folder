# 01 - Directory Scanner

## Summary

Scan a target directory and return a list of all immediate subdirectories.

## Dependencies

- **00-cli-scaffold** — Requires CLI argument parsing to receive target directory path

## Description

This feature implements the directory scanning functionality that reads the target directory and enumerates all immediate subdirectories. It forms the foundation for format detection and validation features.

The scanner should:

- Accept a directory path and validate it exists
- List only immediate subdirectories (not files, not nested directories)
- Return directory names and full paths for further processing
- Handle permission errors and inaccessible directories gracefully

## Requirements

### Functional Requirements

1. Accept a `PathBuf` representing the target directory
2. Verify the path exists and is a directory
3. Return a list of immediate subdirectories with:
   - Directory name (String)
   - Full path (PathBuf)
4. Ignore files in the target directory
5. Ignore hidden directories (starting with `.`)
6. Sort results alphabetically for consistent output

### Non-Functional Requirements

1. Use `std::fs` for filesystem operations
2. Return meaningful errors for:
   - Path does not exist
   - Path is not a directory
   - Permission denied
   - Other I/O errors
3. Efficient memory usage for directories with many subdirectories

## Implementation Guide

### Step 1: Create Scanner Module

```rust
// src/scanner.rs
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),
    
    #[error("Path is not a directory: {0}")]
    NotADirectory(PathBuf),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),
    
    #[error("Failed to read directory: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: PathBuf,
}

impl DirectoryEntry {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }
}
```

### Step 2: Implement Scanner Function

```rust
// src/scanner.rs (continued)

pub fn scan_directory(target: &Path) -> Result<Vec<DirectoryEntry>, ScannerError> {
    // Validate path exists
    if !target.exists() {
        return Err(ScannerError::PathNotFound(target.to_path_buf()));
    }
    
    // Validate path is a directory
    if !target.is_dir() {
        return Err(ScannerError::NotADirectory(target.to_path_buf()));
    }
    
    // Read directory entries
    let mut entries = Vec::new();
    
    let read_dir = fs::read_dir(target).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ScannerError::PermissionDenied(target.to_path_buf())
        } else {
            ScannerError::IoError(e)
        }
    })?;
    
    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();
        
        // Skip if not a directory
        if !path.is_dir() {
            continue;
        }
        
        // Get directory name
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        
        // Skip hidden directories
        if name.starts_with('.') {
            continue;
        }
        
        entries.push(DirectoryEntry::new(name, path));
    }
    
    // Sort alphabetically
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(entries)
}
```

### Step 3: Integrate with Main

```rust
// src/main.rs
mod cli;
mod scanner;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Args;
use scanner::scan_directory;
use tracing::{debug, info};

fn main() -> Result<()> {
    let args = Args::parse();
    // ... logging setup ...
    
    if let Some(target_dir) = &args.target_dir {
        let entries = scan_directory(target_dir)
            .context("Failed to scan target directory")?;
        
        info!("Found {} subdirectories", entries.len());
        
        for entry in &entries {
            debug!("  - {}", entry.name);
        }
        
        // TODO: Pass to format validator (feature 21)
    }
    
    Ok(())
}
```

### Step 4: Add to Library Root

```rust
// src/lib.rs
pub mod scanner;

pub use scanner::{scan_directory, DirectoryEntry, ScannerError};
```

## Test Cases

### Unit Tests

```rust
// src/scanner.rs (tests module)
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    
    #[test]
    fn test_scan_empty_directory() {
        let dir = tempdir().unwrap();
        let result = scan_directory(dir.path()).unwrap();
        assert!(result.is_empty());
    }
    
    #[test]
    fn test_scan_with_subdirectories() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("subdir1")).unwrap();
        fs::create_dir(dir.path().join("subdir2")).unwrap();
        
        let result = scan_directory(dir.path()).unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "subdir1");
        assert_eq!(result[1].name, "subdir2");
    }
    
    #[test]
    fn test_ignores_files() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();
        
        let result = scan_directory(dir.path()).unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "subdir");
    }
    
    #[test]
    fn test_ignores_hidden_directories() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".hidden")).unwrap();
        fs::create_dir(dir.path().join("visible")).unwrap();
        
        let result = scan_directory(dir.path()).unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "visible");
    }
    
    #[test]
    fn test_path_not_found() {
        let result = scan_directory(Path::new("/nonexistent/path"));
        assert!(matches!(result, Err(ScannerError::PathNotFound(_))));
    }
    
    #[test]
    fn test_not_a_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();
        
        let result = scan_directory(&file_path);
        assert!(matches!(result, Err(ScannerError::NotADirectory(_))));
    }
    
    #[test]
    fn test_alphabetical_sorting() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("zebra")).unwrap();
        fs::create_dir(dir.path().join("alpha")).unwrap();
        fs::create_dir(dir.path().join("beta")).unwrap();
        
        let result = scan_directory(dir.path()).unwrap();
        
        assert_eq!(result[0].name, "alpha");
        assert_eq!(result[1].name, "beta");
        assert_eq!(result[2].name, "zebra");
    }
}
```

### Integration Tests

```rust
// tests/scanner_tests.rs
use anidb2folder::scan_directory;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_scan_realistic_structure() {
    let dir = tempdir().unwrap();
    
    // Create AniDB-style directories
    fs::create_dir(dir.path().join("[AS0] 12345")).unwrap();
    fs::create_dir(dir.path().join("67890")).unwrap();
    fs::create_dir(dir.path().join("[Series] 11111")).unwrap();
    
    // Create a file that should be ignored
    fs::write(dir.path().join("readme.txt"), "content").unwrap();
    
    let result = scan_directory(dir.path()).unwrap();
    
    assert_eq!(result.len(), 3);
}
```

## Notes

- The scanner is intentionally simple and only handles directory listing
- Format detection and validation are handled by separate features (20, 21)
- Consider adding symlink handling in the future
- The `tempfile` crate is used for testing to create temporary directories
