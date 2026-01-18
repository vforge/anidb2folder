pub mod api;
pub mod cache;
pub mod cli;
pub mod error;
pub mod logging;
pub mod output;
pub mod parser;
pub mod rename;
pub mod scanner;
pub mod validator;

pub use api::{
    config_from_env, AniDbClient, AnimeInfo, ApiConfig, ApiError, ENV_ANIDB_CLIENT,
    ENV_ANIDB_CLIENT_VERSION,
};
pub use cache::{CacheConfig, CacheEntry, CacheError, CacheStore, CACHE_VERSION};
pub use error::{AppError, ExitCode};
pub use output::{display_dry_run, display_dry_run_simple, display_execution_result};
pub use parser::{
    detect_format, is_anidb_format, is_human_readable_format, parse_directory_name, AniDbFormat,
    DirectoryFormat, HumanReadableFormat, ParseError, ParsedDirectory,
};
pub use rename::{RenameDirection, RenameOperation, RenameResult};
pub use scanner::{scan_directory, DirectoryEntry, ScannerError};
pub use validator::{validate_directories, FormatMismatch, ValidationError, ValidationResult};
