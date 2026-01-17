# 03 - Error Handling

## Summary

Implement standardized error codes and user-friendly error messages.

## Dependencies

- **00-cli-scaffold** — Base CLI infrastructure for error output

## Description

This feature implements a comprehensive error handling system with standardized exit codes and clear, actionable error messages. Proper error handling is critical for:

- Scripting and automation (exit codes)
- User understanding of failures
- Debugging issues
- Graceful degradation

## Requirements

### Functional Requirements

1. Define standardized exit codes for all error conditions
2. Provide clear, human-readable error messages
3. Include context in error messages (file paths, IDs, etc.)
4. Suggest corrective actions where possible
5. Support both brief and detailed error output

### Non-Functional Requirements

1. Use `thiserror` for error type definitions
2. Use `anyhow` for error propagation in main
3. Exit codes should be documented and stable
4. Errors should be written to stderr

## Exit Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | SUCCESS | Operation completed successfully |
| 1 | GENERAL_ERROR | Unspecified error |
| 2 | INVALID_ARGUMENTS | Invalid command-line arguments |
| 3 | DIRECTORY_NOT_FOUND | Target directory does not exist |
| 4 | MIXED_FORMATS | Directories have mixed formats |
| 5 | UNRECOGNIZED_FORMAT | Some directories don't match known formats |
| 6 | API_ERROR | Failed to fetch data from AniDB API |
| 7 | PERMISSION_ERROR | Insufficient filesystem permissions |
| 8 | HISTORY_ERROR | History file not found or corrupted |
| 9 | RENAME_ERROR | Failed to rename directory |
| 10 | CACHE_ERROR | Cache read/write failure |

## Implementation Guide

### Step 1: Define Exit Codes

```rust
// src/error/codes.rs

/// Exit codes for the application
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    InvalidArguments = 2,
    DirectoryNotFound = 3,
    MixedFormats = 4,
    UnrecognizedFormat = 5,
    ApiError = 6,
    PermissionError = 7,
    HistoryError = 8,
    RenameError = 9,
    CacheError = 10,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}
```

### Step 2: Define Application Error Type

```rust
// src/error/mod.rs
mod codes;

pub use codes::ExitCode;

use thiserror::Error;
use std::path::PathBuf;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Target directory not found: {path}")]
    DirectoryNotFound {
        path: PathBuf,
    },
    
    #[error("Permission denied: {path}")]
    PermissionDenied {
        path: PathBuf,
    },
    
    #[error("Mixed directory formats found")]
    MixedFormats {
        anidb_count: usize,
        readable_count: usize,
        anidb_examples: Vec<String>,
        readable_examples: Vec<String>,
    },
    
    #[error("Unrecognized directory format")]
    UnrecognizedFormat {
        directories: Vec<String>,
    },
    
    #[error("API error for anime {anidb_id}: {message}")]
    ApiError {
        anidb_id: u32,
        message: String,
    },
    
    #[error("History file error: {message}")]
    HistoryError {
        path: Option<PathBuf>,
        message: String,
    },
    
    #[error("Rename failed: {from} -> {to}")]
    RenameError {
        from: String,
        to: String,
        source: std::io::Error,
    },
    
    #[error("Cache error: {message}")]
    CacheError {
        message: String,
    },
    
    #[error("{message}")]
    Other {
        message: String,
    },
}

impl AppError {
    /// Get the exit code for this error
    pub fn exit_code(&self) -> ExitCode {
        match self {
            AppError::DirectoryNotFound { .. } => ExitCode::DirectoryNotFound,
            AppError::PermissionDenied { .. } => ExitCode::PermissionError,
            AppError::MixedFormats { .. } => ExitCode::MixedFormats,
            AppError::UnrecognizedFormat { .. } => ExitCode::UnrecognizedFormat,
            AppError::ApiError { .. } => ExitCode::ApiError,
            AppError::HistoryError { .. } => ExitCode::HistoryError,
            AppError::RenameError { .. } => ExitCode::RenameError,
            AppError::CacheError { .. } => ExitCode::CacheError,
            AppError::Other { .. } => ExitCode::GeneralError,
        }
    }
    
    /// Get a detailed, user-friendly error message
    pub fn detailed_message(&self) -> String {
        match self {
            AppError::DirectoryNotFound { path } => {
                format!(
                    "The specified directory does not exist:\n  {}\n\n\
                     Please verify the path and try again.",
                    path.display()
                )
            }
            
            AppError::PermissionDenied { path } => {
                format!(
                    "Permission denied when accessing:\n  {}\n\n\
                     Please check file permissions or run with appropriate privileges.",
                    path.display()
                )
            }
            
            AppError::MixedFormats { 
                anidb_count, 
                readable_count, 
                anidb_examples, 
                readable_examples 
            } => {
                let mut msg = format!(
                    "Found directories in multiple formats:\n\
                     - {} in AniDB format\n\
                     - {} in human-readable format\n\n",
                    anidb_count, readable_count
                );
                
                if !anidb_examples.is_empty() {
                    msg.push_str("AniDB format examples:\n");
                    for ex in anidb_examples.iter().take(3) {
                        msg.push_str(&format!("  - {}\n", ex));
                    }
                }
                
                if !readable_examples.is_empty() {
                    msg.push_str("\nHuman-readable format examples:\n");
                    for ex in readable_examples.iter().take(3) {
                        msg.push_str(&format!("  - {}\n", ex));
                    }
                }
                
                msg.push_str("\nAll directories must be in the same format.\n\
                              Manually rename mixed directories before running again.");
                msg
            }
            
            AppError::UnrecognizedFormat { directories } => {
                let mut msg = String::from(
                    "The following directories do not match any known format:\n"
                );
                for dir in directories.iter().take(10) {
                    msg.push_str(&format!("  - {}\n", dir));
                }
                if directories.len() > 10 {
                    msg.push_str(&format!("  ... and {} more\n", directories.len() - 10));
                }
                msg.push_str("\nExpected formats:\n");
                msg.push_str("  AniDB:          [<series>] <anidb_id>\n");
                msg.push_str("                  Examples: 12345, [AS0] 67890\n");
                msg.push_str("  Human-readable: <title> (<year>) [anidb-<id>]\n");
                msg.push_str("                  Examples: Naruto (2002) [anidb-12345]\n");
                msg
            }
            
            AppError::ApiError { anidb_id, message } => {
                format!(
                    "Failed to fetch data for anime ID {}:\n  {}\n\n\
                     This could be due to:\n\
                     - Network connectivity issues\n\
                     - AniDB API rate limiting\n\
                     - Invalid anime ID\n\n\
                     Try again later or check your internet connection.",
                    anidb_id, message
                )
            }
            
            AppError::HistoryError { path, message } => {
                let path_info = path.as_ref()
                    .map(|p| format!("File: {}\n", p.display()))
                    .unwrap_or_default();
                
                format!(
                    "History file error:\n  {}\n{}\n\
                     Ensure the history file exists and is valid JSON.",
                    message, path_info
                )
            }
            
            AppError::RenameError { from, to, source } => {
                format!(
                    "Failed to rename directory:\n\
                     From: {}\n\
                     To:   {}\n\
                     Error: {}\n\n\
                     Check file permissions and ensure no files are open.",
                    from, to, source
                )
            }
            
            AppError::CacheError { message } => {
                format!(
                    "Cache error: {}\n\n\
                     The cache file may be corrupted. \
                     Delete the cache file to rebuild it.",
                    message
                )
            }
            
            AppError::Other { message } => {
                message.clone()
            }
        }
    }
    
    /// Get a brief error message suitable for logging
    pub fn brief_message(&self) -> String {
        self.to_string()
    }
}
```

### Step 3: Error Formatting and Output

```rust
// src/error/output.rs
use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use super::AppError;

/// Print error to stderr with formatting
pub fn print_error(error: &AppError, detailed: bool) {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    
    // Error header
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)).ok();
    write!(stderr, "Error: ").ok();
    stderr.reset().ok();
    
    // Error message
    if detailed {
        writeln!(stderr, "\n{}", error.detailed_message()).ok();
    } else {
        writeln!(stderr, "{}", error.brief_message()).ok();
    }
    
    // Exit code hint (for scripting)
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).ok();
    writeln!(stderr, "\nExit code: {}", error.exit_code() as i32).ok();
    stderr.reset().ok();
}

/// Print error and exit with appropriate code
pub fn exit_with_error(error: AppError) -> ! {
    print_error(&error, true);
    std::process::exit(error.exit_code().into())
}
```

### Step 4: Integration with Main

```rust
// src/main.rs
use crate::error::{AppError, ExitCode, exit_with_error};

fn main() {
    if let Err(e) = run() {
        exit_with_error(e);
    }
}

fn run() -> Result<(), AppError> {
    let args = Args::parse();
    
    // Validate target directory
    if let Some(ref target) = args.target_dir {
        if !target.exists() {
            return Err(AppError::DirectoryNotFound {
                path: target.clone(),
            });
        }
    }
    
    // ... rest of logic ...
    
    Ok(())
}
```

### Step 5: Conversion from Other Error Types

```rust
// src/error/conversions.rs
use super::AppError;
use crate::api::ApiError;
use crate::scanner::ScannerError;
use crate::validator::ValidationError;

impl From<ScannerError> for AppError {
    fn from(err: ScannerError) -> Self {
        match err {
            ScannerError::PathNotFound(path) => AppError::DirectoryNotFound { path },
            ScannerError::NotADirectory(path) => AppError::DirectoryNotFound { path },
            ScannerError::PermissionDenied(path) => AppError::PermissionDenied { path },
            ScannerError::IoError(e) => AppError::Other {
                message: format!("I/O error: {}", e),
            },
        }
    }
}

impl From<ValidationError> for AppError {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::UnrecognizedDirectories { directories } => {
                AppError::UnrecognizedFormat { directories }
            }
            ValidationError::MixedFormats { mismatch } => {
                AppError::MixedFormats {
                    anidb_count: mismatch.anidb_dirs.len(),
                    readable_count: mismatch.human_readable_dirs.len(),
                    anidb_examples: mismatch.anidb_dirs,
                    readable_examples: mismatch.human_readable_dirs,
                }
            }
            ValidationError::NoDirectories => AppError::Other {
                message: "No directories found in target".to_string(),
            },
        }
    }
}

impl From<ApiError> for AppError {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::NotFound(id) => AppError::ApiError {
                anidb_id: id,
                message: "Anime not found in AniDB".to_string(),
            },
            _ => AppError::ApiError {
                anidb_id: 0,
                message: err.to_string(),
            },
        }
    }
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_exit_codes() {
        let err = AppError::DirectoryNotFound {
            path: PathBuf::from("/test"),
        };
        assert_eq!(err.exit_code(), ExitCode::DirectoryNotFound);
        
        let err = AppError::MixedFormats {
            anidb_count: 1,
            readable_count: 1,
            anidb_examples: vec![],
            readable_examples: vec![],
        };
        assert_eq!(err.exit_code(), ExitCode::MixedFormats);
    }
    
    #[test]
    fn test_detailed_message_includes_context() {
        let err = AppError::UnrecognizedFormat {
            directories: vec!["dir1".to_string(), "dir2".to_string()],
        };
        
        let msg = err.detailed_message();
        assert!(msg.contains("dir1"));
        assert!(msg.contains("dir2"));
        assert!(msg.contains("Expected formats"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_exit_code_directory_not_found() {
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.arg("/nonexistent/path")
        .assert()
        .code(3); // DIRECTORY_NOT_FOUND
}

#[test]
fn test_exit_code_invalid_arguments() {
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.arg("--invalid-flag")
        .assert()
        .code(2); // INVALID_ARGUMENTS
}
```

## Notes

- Exit codes are stable and documented for scripting
- The `detailed_message()` is shown by default; `brief_message()` for logs
- Consider adding `--quiet` flag to suppress detailed messages
- Error messages should be actionable — tell users what to do
- Use color output when writing to a terminal
