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

impl ValidationError {
    pub fn format_error_message(&self) -> String {
        match self {
            ValidationError::UnrecognizedDirectories { directories } => {
                let mut msg =
                    String::from("The following directories do not match any known format:\n");
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
                    "Found directories in multiple formats. All directories must be in the same format.\n\n",
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
