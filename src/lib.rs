pub mod cli;
pub mod error;
pub mod logging;
pub mod scanner;

pub use error::{AppError, ExitCode};
pub use scanner::{scan_directory, DirectoryEntry, ScannerError};
