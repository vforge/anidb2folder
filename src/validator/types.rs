use crate::parser::{DirectoryFormat, ParsedDirectory};
use thiserror::Error;

#[derive(Debug)]
pub struct ValidationResult {
    pub format: DirectoryFormat,
    pub directories: Vec<ParsedDirectory>,
}

#[derive(Debug, Clone)]
pub struct FormatMismatch {
    pub anidb_dirs: Vec<String>,
    pub human_readable_dirs: Vec<String>,
}

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Unrecognized directory format")]
    UnrecognizedDirectories { directories: Vec<String> },

    #[error("Mixed directory formats found")]
    MixedFormats { mismatch: FormatMismatch },

    #[error("No directories found in target")]
    NoDirectories,
}
