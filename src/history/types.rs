use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const HISTORY_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryFile {
    /// Schema version for compatibility
    pub version: String,

    /// When the operation was executed
    pub executed_at: DateTime<Utc>,

    /// Type of operation performed
    pub operation: OperationType,

    /// Direction of rename
    pub direction: HistoryDirection,

    /// Target directory path
    pub target_directory: PathBuf,

    /// Tool version that created this history
    pub tool_version: String,

    /// All changes made
    pub changes: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Rename,
    Revert,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HistoryDirection {
    AnidbToReadable,
    ReadableToAnidb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Original directory name
    pub source: String,

    /// New directory name
    pub destination: String,

    /// AniDB ID for the anime
    pub anidb_id: u32,

    /// Whether the name was truncated
    pub truncated: bool,
}

impl HistoryFile {
    /// Generate the filename for this history file
    pub fn generate_filename(&self) -> String {
        let timestamp = self.executed_at.format("%Y%m%d-%H%M%S");
        format!("anidb2folder-history-{}.json", timestamp)
    }
}

impl HistoryDirection {
    pub fn description(&self) -> &'static str {
        match self {
            HistoryDirection::AnidbToReadable => "AniDB -> Human-readable",
            HistoryDirection::ReadableToAnidb => "Human-readable -> AniDB",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_filename() {
        let history = HistoryFile {
            version: HISTORY_VERSION.to_string(),
            executed_at: DateTime::parse_from_rfc3339("2026-01-15T10:30:45Z")
                .unwrap()
                .with_timezone(&Utc),
            operation: OperationType::Rename,
            direction: HistoryDirection::AnidbToReadable,
            target_directory: PathBuf::from("/test"),
            tool_version: "0.1.0".to_string(),
            changes: vec![],
        };

        assert_eq!(
            history.generate_filename(),
            "anidb2folder-history-20260115-103045.json"
        );
    }

    #[test]
    fn test_history_direction_description() {
        assert_eq!(
            HistoryDirection::AnidbToReadable.description(),
            "AniDB -> Human-readable"
        );
        assert_eq!(
            HistoryDirection::ReadableToAnidb.description(),
            "Human-readable -> AniDB"
        );
    }

    #[test]
    fn test_operation_type_serialization() {
        assert_eq!(
            serde_json::to_string(&OperationType::Rename).unwrap(),
            "\"rename\""
        );
        assert_eq!(
            serde_json::to_string(&OperationType::Revert).unwrap(),
            "\"revert\""
        );
    }

    #[test]
    fn test_direction_serialization() {
        assert_eq!(
            serde_json::to_string(&HistoryDirection::AnidbToReadable).unwrap(),
            "\"anidb_to_readable\""
        );
        assert_eq!(
            serde_json::to_string(&HistoryDirection::ReadableToAnidb).unwrap(),
            "\"readable_to_anidb\""
        );
    }
}
