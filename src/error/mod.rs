mod codes;

pub use codes::ExitCode;

use crate::scanner::ScannerError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Target directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    #[error("Path is not a directory: {path}")]
    NotADirectory { path: PathBuf },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Mixed directory formats found")]
    MixedFormats {
        anidb_count: usize,
        readable_count: usize,
        anidb_examples: Vec<String>,
        readable_examples: Vec<String>,
    },

    #[error("Unrecognized directory format")]
    UnrecognizedFormat { directories: Vec<String> },

    #[error("API error for anime {anidb_id}: {message}")]
    ApiError { anidb_id: u32, message: String },

    #[error("History file error: {message}")]
    HistoryError {
        path: Option<PathBuf>,
        message: String,
    },

    #[error("Rename failed: {from} -> {to}")]
    RenameError {
        from: String,
        to: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Cache error: {message}")]
    CacheError { message: String },

    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            AppError::DirectoryNotFound { .. } => ExitCode::DirectoryNotFound,
            AppError::NotADirectory { .. } => ExitCode::DirectoryNotFound,
            AppError::PermissionDenied { .. } => ExitCode::PermissionError,
            AppError::MixedFormats { .. } => ExitCode::MixedFormats,
            AppError::UnrecognizedFormat { .. } => ExitCode::UnrecognizedFormat,
            AppError::ApiError { .. } => ExitCode::ApiError,
            AppError::HistoryError { .. } => ExitCode::HistoryError,
            AppError::RenameError { .. } => ExitCode::RenameError,
            AppError::CacheError { .. } => ExitCode::CacheError,
            AppError::Other(_) => ExitCode::GeneralError,
        }
    }

    pub fn detailed_message(&self) -> String {
        match self {
            AppError::DirectoryNotFound { path } => {
                format!(
                    "The specified directory does not exist:\n  {}\n\n\
                     Please verify the path and try again.",
                    path.display()
                )
            }

            AppError::NotADirectory { path } => {
                format!(
                    "The specified path is not a directory:\n  {}\n\n\
                     Please provide a valid directory path.",
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
                readable_examples,
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

                msg.push_str(
                    "\nAll directories must be in the same format.\n\
                     Manually rename mixed directories before running again.",
                );
                msg
            }

            AppError::UnrecognizedFormat { directories } => {
                let mut msg =
                    String::from("The following directories do not match any known format:\n");
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
                let path_info = path
                    .as_ref()
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

            AppError::Other(message) => message.clone(),
        }
    }
}

impl From<ScannerError> for AppError {
    fn from(err: ScannerError) -> Self {
        match err {
            ScannerError::PathNotFound(path) => AppError::DirectoryNotFound { path },
            ScannerError::NotADirectory(path) => AppError::NotADirectory { path },
            ScannerError::PermissionDenied(path) => AppError::PermissionDenied { path },
            ScannerError::IoError(e) => AppError::Other(format!("I/O error: {}", e)),
        }
    }
}

impl From<crate::validator::ValidationError> for AppError {
    fn from(err: crate::validator::ValidationError) -> Self {
        use crate::validator::ValidationError;
        match err {
            ValidationError::UnrecognizedDirectories { directories } => {
                AppError::UnrecognizedFormat { directories }
            }
            ValidationError::MixedFormats { mismatch } => AppError::MixedFormats {
                anidb_count: mismatch.anidb_dirs.len(),
                readable_count: mismatch.human_readable_dirs.len(),
                anidb_examples: mismatch.anidb_dirs,
                readable_examples: mismatch.human_readable_dirs,
            },
            ValidationError::NoDirectories => {
                AppError::Other("No subdirectories found in target".to_string())
            }
        }
    }
}

impl From<crate::api::ApiError> for AppError {
    fn from(err: crate::api::ApiError) -> Self {
        use crate::api::ApiError;
        match err {
            ApiError::NotFound(id) => AppError::ApiError {
                anidb_id: id,
                message: "Anime not found".to_string(),
            },
            ApiError::RateLimited => AppError::ApiError {
                anidb_id: 0,
                message: "Rate limited by AniDB - please wait and try again".to_string(),
            },
            ApiError::NetworkError(msg) => AppError::ApiError {
                anidb_id: 0,
                message: format!("Network error: {}", msg),
            },
            ApiError::Timeout => AppError::ApiError {
                anidb_id: 0,
                message: "Request timed out".to_string(),
            },
            ApiError::ParseError(msg) => AppError::ApiError {
                anidb_id: 0,
                message: format!("Failed to parse response: {}", msg),
            },
            ApiError::ServerError(msg) => AppError::ApiError {
                anidb_id: 0,
                message: format!("API error: {}", msg),
            },
            ApiError::MaxRetriesExceeded { attempts } => AppError::ApiError {
                anidb_id: 0,
                message: format!("Max retries ({}) exceeded", attempts),
            },
            ApiError::NotConfigured => AppError::ApiError {
                anidb_id: 0,
                message: "API client not configured. Set ANIDB_CLIENT and ANIDB_CLIENT_VERSION environment variables or create a .env file".to_string(),
            },
            ApiError::Banned(msg) => AppError::ApiError {
                anidb_id: 0,
                message: format!("Banned by AniDB: {}", msg),
            },
        }
    }
}

impl From<crate::cache::CacheError> for AppError {
    fn from(err: crate::cache::CacheError) -> Self {
        AppError::CacheError {
            message: err.to_string(),
        }
    }
}

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

        let err = AppError::PermissionDenied {
            path: PathBuf::from("/test"),
        };
        assert_eq!(err.exit_code(), ExitCode::PermissionError);
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

    #[test]
    fn test_scanner_error_conversion() {
        let scanner_err = ScannerError::PathNotFound(PathBuf::from("/missing"));
        let app_err: AppError = scanner_err.into();
        assert_eq!(app_err.exit_code(), ExitCode::DirectoryNotFound);
    }
}
