mod types;

pub use types::*;

use crate::parser::{parse_directory_name, DirectoryFormat, ParsedDirectory};
use crate::scanner::DirectoryEntry;
use tracing::{debug, info, warn};

/// Validate that all directories are in the same format
pub fn validate_directories(
    entries: &[DirectoryEntry],
) -> Result<ValidationResult, ValidationError> {
    if entries.is_empty() {
        return Err(ValidationError::NoDirectories);
    }

    info!("Validating {} directories", entries.len());

    let mut parsed: Vec<ParsedDirectory> = Vec::with_capacity(entries.len());
    let mut unrecognized: Vec<String> = Vec::new();
    let mut anidb_dirs: Vec<String> = Vec::new();
    let mut human_readable_dirs: Vec<String> = Vec::new();

    for entry in entries {
        match parse_directory_name(&entry.name) {
            Ok(p) => {
                debug!(name = %entry.name, format = ?p.format(), "Parsed directory");

                match p.format() {
                    DirectoryFormat::AniDb => anidb_dirs.push(entry.name.clone()),
                    DirectoryFormat::HumanReadable => human_readable_dirs.push(entry.name.clone()),
                }

                parsed.push(p);
            }
            Err(_) => {
                debug!(name = %entry.name, "Unrecognized format");
                unrecognized.push(entry.name.clone());
            }
        }
    }

    if !unrecognized.is_empty() {
        warn!(
            count = unrecognized.len(),
            "Directories with unrecognized format"
        );
        return Err(ValidationError::UnrecognizedDirectories {
            directories: unrecognized,
        });
    }

    let has_anidb = !anidb_dirs.is_empty();
    let has_human_readable = !human_readable_dirs.is_empty();

    if has_anidb && has_human_readable {
        warn!(
            anidb = anidb_dirs.len(),
            human_readable = human_readable_dirs.len(),
            "Mixed formats detected"
        );
        return Err(ValidationError::MixedFormats {
            mismatch: FormatMismatch {
                anidb_dirs,
                human_readable_dirs,
            },
        });
    }

    let format = if has_anidb {
        DirectoryFormat::AniDb
    } else {
        DirectoryFormat::HumanReadable
    };

    info!(
        count = parsed.len(),
        format = ?format,
        "Validation passed"
    );

    Ok(ValidationResult {
        format,
        directories: parsed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str) -> DirectoryEntry {
        DirectoryEntry {
            name: name.to_string(),
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
            make_entry("12345"),
            make_entry("Naruto (2002) [anidb-67890]"),
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
            make_entry("Random Folder"),
            make_entry("Another Invalid"),
        ];

        let result = validate_directories(&entries);

        assert!(matches!(
            result,
            Err(ValidationError::UnrecognizedDirectories { .. })
        ));

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
}
