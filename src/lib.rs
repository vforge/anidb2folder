pub mod cli;
pub mod error;
pub mod logging;
pub mod parser;
pub mod scanner;
pub mod validator;

pub use error::{AppError, ExitCode};
pub use parser::{
    detect_format, is_anidb_format, is_human_readable_format, parse_directory_name, AniDbFormat,
    DirectoryFormat, HumanReadableFormat, ParseError, ParsedDirectory,
};
pub use scanner::{scan_directory, DirectoryEntry, ScannerError};
pub use validator::{validate_directories, FormatMismatch, ValidationError, ValidationResult};
