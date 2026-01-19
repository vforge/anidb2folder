pub mod api;
pub mod cache;
pub mod cli;
pub mod error;
pub mod history;
pub mod logging;
pub mod parser;
pub mod progress;
pub mod rename;
pub mod revert;
pub mod scanner;
pub mod ui;
pub mod validator;

pub use api::{
    config_from_env, AniDbClient, AnimeInfo, ApiConfig, ApiError, ENV_ANIDB_CLIENT,
    ENV_ANIDB_CLIENT_VERSION,
};
pub use cache::{CacheConfig, CacheError, CacheStore};
pub use error::{AppError, ExitCode};
pub use parser::{
    parse_directory_name, AniDbFormat, DirectoryFormat, HumanReadableFormat, ParseError,
    ParsedDirectory,
};
pub use progress::Progress;
pub use rename::{
    build_anidb_name, rename_to_readable, RenameDirection, RenameError, RenameOperation,
    RenameOptions, RenameResult,
};
pub use scanner::{scan_directory, DirectoryEntry, ScannerError};
pub use validator::{validate_directories, FormatMismatch, ValidationError, ValidationResult};
// validate_for_revert: TODO(feature-60) - revert safety validation
#[allow(unused_imports)]
pub use history::{
    read_history, validate_for_revert, write_history, HistoryDirection, HistoryEntry, HistoryError,
    HistoryFile, OperationType, HISTORY_VERSION,
};
pub use revert::{revert_from_history, RevertError, RevertOperation, RevertOptions, RevertResult};
pub use ui::{Ui, UiConfig};
