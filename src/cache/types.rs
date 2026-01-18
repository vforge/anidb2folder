use crate::api::AnimeInfo;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

pub const CACHE_VERSION: &str = "1.0";

/// A single cached anime entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub anidb_id: u32,
    pub title_main: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
    pub fetched_at: DateTime<Utc>,
}

impl CacheEntry {
    pub fn from_anime_info(info: &AnimeInfo) -> Self {
        Self {
            anidb_id: info.anidb_id,
            title_main: info.title_main.clone(),
            title_en: info.title_en.clone(),
            release_year: info.release_year,
            fetched_at: Utc::now(),
        }
    }

    pub fn to_anime_info(&self) -> AnimeInfo {
        AnimeInfo {
            anidb_id: self.anidb_id,
            title_main: self.title_main.clone(),
            title_en: self.title_en.clone(),
            release_year: self.release_year,
        }
    }

    pub fn is_expired(&self, expiry_days: u32) -> bool {
        let age = Utc::now().signed_duration_since(self.fetched_at);
        age.num_days() > expiry_days as i64
    }
}

/// The cache file structure (serialized to JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheFile {
    pub version: String,
    pub entries: HashMap<u32, CacheEntry>,
}

impl Default for CacheFile {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION.to_string(),
            entries: HashMap::new(),
        }
    }
}

/// Configuration for the cache store
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub expiry_days: u32,
    pub cache_path: PathBuf,
}

impl CacheConfig {
    /// Create config for target directory cache
    pub fn for_target_dir(target: &std::path::Path, expiry_days: u32) -> Self {
        Self {
            expiry_days,
            cache_path: target.join(".anidb2folder-cache.json"),
        }
    }

    /// Create config for user home cache directory
    ///
    /// TODO(feature-61): Global cache option (--global-cache)
    #[allow(dead_code)]
    pub fn for_user_home(expiry_days: u32) -> Option<Self> {
        dirs::cache_dir().map(|cache_dir| Self {
            expiry_days,
            cache_path: cache_dir.join("anidb2folder").join("cache.json"),
        })
    }
}

/// Errors that can occur during cache operations
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache file corrupted")]
    Corrupted,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    SerializeError(#[from] serde_json::Error),

    #[error("Cache version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_info(id: u32) -> AnimeInfo {
        AnimeInfo {
            anidb_id: id,
            title_main: format!("Test Anime {}", id),
            title_en: Some(format!("Test Anime {} EN", id)),
            release_year: Some(2020),
        }
    }

    #[test]
    fn test_cache_entry_from_anime_info() {
        let info = create_test_info(12345);
        let entry = CacheEntry::from_anime_info(&info);

        assert_eq!(entry.anidb_id, 12345);
        assert_eq!(entry.title_main, "Test Anime 12345");
        assert_eq!(entry.title_en, Some("Test Anime 12345 EN".to_string()));
        assert_eq!(entry.release_year, Some(2020));
    }

    #[test]
    fn test_cache_entry_to_anime_info() {
        let entry = CacheEntry {
            anidb_id: 1,
            title_main: "Test".to_string(),
            title_en: Some("Test EN".to_string()),
            release_year: Some(2000),
            fetched_at: Utc::now(),
        };

        let info = entry.to_anime_info();

        assert_eq!(info.anidb_id, 1);
        assert_eq!(info.title_main, "Test");
        assert_eq!(info.title_en, Some("Test EN".to_string()));
        assert_eq!(info.release_year, Some(2000));
    }

    #[test]
    fn test_cache_entry_expiration() {
        let mut entry = CacheEntry {
            anidb_id: 1,
            title_main: "Test".to_string(),
            title_en: None,
            release_year: None,
            fetched_at: Utc::now() - Duration::days(31),
        };

        // 31 days old with 30 day expiry = expired
        assert!(entry.is_expired(30));

        // 31 days old with 60 day expiry = not expired
        assert!(!entry.is_expired(60));

        // Fresh entry = not expired
        entry.fetched_at = Utc::now();
        assert!(!entry.is_expired(30));
    }

    #[test]
    fn test_cache_file_default() {
        let cache = CacheFile::default();

        assert_eq!(cache.version, CACHE_VERSION);
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_cache_config_for_target_dir() {
        let target = std::path::Path::new("/tmp/anime");
        let config = CacheConfig::for_target_dir(target, 30);

        assert_eq!(config.expiry_days, 30);
        assert_eq!(
            config.cache_path,
            std::path::PathBuf::from("/tmp/anime/.anidb2folder-cache.json")
        );
    }

    #[test]
    fn test_cache_config_for_user_home() {
        let config = CacheConfig::for_user_home(15);

        // Should return Some on most systems
        if let Some(c) = config {
            assert_eq!(c.expiry_days, 15);
            assert!(c.cache_path.to_string_lossy().contains("anidb2folder"));
        }
    }

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::Corrupted;
        assert!(err.to_string().contains("corrupted"));

        let err = CacheError::VersionMismatch {
            expected: "1.0".to_string(),
            found: "2.0".to_string(),
        };
        assert!(err.to_string().contains("1.0"));
        assert!(err.to_string().contains("2.0"));
    }
}
