use std::path::PathBuf;

/// Direction of the rename operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameDirection {
    /// Converting from AniDB format to human-readable format
    AniDbToReadable,
    /// Converting from human-readable format to AniDB format
    ReadableToAniDb,
}

impl RenameDirection {
    pub fn description(&self) -> &'static str {
        match self {
            RenameDirection::AniDbToReadable => "AniDB → Human-readable",
            RenameDirection::ReadableToAniDb => "Human-readable → AniDB",
        }
    }
}

/// A single rename operation
#[derive(Debug, Clone)]
pub struct RenameOperation {
    /// Full path to the source directory
    pub source_path: PathBuf,
    /// Original directory name
    pub source_name: String,
    /// Full path to the destination
    pub destination_path: PathBuf,
    /// New directory name
    pub destination_name: String,
    /// AniDB ID extracted from the directory
    pub anidb_id: u32,
    /// Whether the name was truncated to fit filesystem limits
    pub truncated: bool,
}

impl RenameOperation {
    pub fn new(
        source_path: PathBuf,
        destination_name: String,
        anidb_id: u32,
        truncated: bool,
    ) -> Self {
        let source_name = source_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let destination_path = source_path
            .parent()
            .map(|p| p.join(&destination_name))
            .unwrap_or_else(|| PathBuf::from(&destination_name));

        Self {
            source_path,
            source_name,
            destination_path,
            destination_name,
            anidb_id,
            truncated,
        }
    }
}

/// Result of a rename batch operation
#[derive(Debug, Clone)]
pub struct RenameResult {
    /// Direction of the rename
    pub direction: RenameDirection,
    /// List of operations performed or planned
    pub operations: Vec<RenameOperation>,
    /// Whether this was a dry run
    pub dry_run: bool,
}

impl RenameResult {
    pub fn new(direction: RenameDirection, dry_run: bool) -> Self {
        Self {
            direction,
            operations: Vec::new(),
            dry_run,
        }
    }

    pub fn add_operation(&mut self, op: RenameOperation) {
        self.operations.push(op);
    }

    /// TODO(feature-62): Report truncated count in UI output
    #[allow(dead_code)]
    pub fn truncated_count(&self) -> usize {
        self.operations.iter().filter(|op| op.truncated).count()
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.operations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rename_direction_description() {
        assert_eq!(
            RenameDirection::AniDbToReadable.description(),
            "AniDB → Human-readable"
        );
        assert_eq!(
            RenameDirection::ReadableToAniDb.description(),
            "Human-readable → AniDB"
        );
    }

    #[test]
    fn test_rename_operation_new() {
        let op = RenameOperation::new(
            PathBuf::from("/anime/12345"),
            "Cowboy Bebop (1998) [anidb-1]".to_string(),
            1,
            false,
        );

        assert_eq!(op.source_name, "12345");
        assert_eq!(op.destination_name, "Cowboy Bebop (1998) [anidb-1]");
        assert_eq!(op.anidb_id, 1);
        assert!(!op.truncated);
        assert_eq!(
            op.destination_path,
            PathBuf::from("/anime/Cowboy Bebop (1998) [anidb-1]")
        );
    }

    #[test]
    fn test_rename_result() {
        let mut result = RenameResult::new(RenameDirection::AniDbToReadable, true);

        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
        assert!(result.dry_run);

        result.add_operation(RenameOperation::new(
            PathBuf::from("/anime/1"),
            "Test [anidb-1]".to_string(),
            1,
            false,
        ));

        result.add_operation(RenameOperation::new(
            PathBuf::from("/anime/2"),
            "Truncated... [anidb-2]".to_string(),
            2,
            true,
        ));

        assert!(!result.is_empty());
        assert_eq!(result.len(), 2);
        assert_eq!(result.truncated_count(), 1);
    }
}
