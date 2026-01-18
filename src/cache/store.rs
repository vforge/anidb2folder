use super::types::{CacheConfig, CacheEntry, CacheError, CacheFile, CACHE_VERSION};
use crate::api::AnimeInfo;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use tracing::{debug, info, warn};

/// A persistent cache store for anime metadata
pub struct CacheStore {
    config: CacheConfig,
    data: CacheFile,
    dirty: bool,
}

impl CacheStore {
    /// Load cache from disk or create new empty cache
    pub fn load(config: CacheConfig) -> Self {
        let data = match Self::read_cache_file(&config.cache_path) {
            Ok(cache) => {
                info!("Loaded cache with {} entries", cache.entries.len());
                cache
            }
            Err(e) => {
                match &e {
                    CacheError::IoError(io_err)
                        if io_err.kind() == std::io::ErrorKind::NotFound =>
                    {
                        debug!("No cache file found, starting fresh");
                    }
                    _ => {
                        warn!("Failed to load cache: {}, starting fresh", e);
                    }
                }
                CacheFile::default()
            }
        };

        Self {
            config,
            data,
            dirty: false,
        }
    }

    fn read_cache_file(path: &Path) -> Result<CacheFile, CacheError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let cache: CacheFile =
            serde_json::from_reader(reader).map_err(|_| CacheError::Corrupted)?;

        // Version check
        if cache.version != CACHE_VERSION {
            return Err(CacheError::VersionMismatch {
                expected: CACHE_VERSION.to_string(),
                found: cache.version,
            });
        }

        Ok(cache)
    }

    /// Get cached anime info if it exists and is not expired
    pub fn get(&self, anidb_id: u32) -> Option<AnimeInfo> {
        self.data.entries.get(&anidb_id).and_then(|entry| {
            if entry.is_expired(self.config.expiry_days) {
                debug!("Cache entry {} expired", anidb_id);
                None
            } else {
                debug!("Cache hit for {}", anidb_id);
                Some(entry.to_anime_info())
            }
        })
    }

    /// Check if a valid (non-expired) entry exists
    ///
    /// TODO(feature-61): Cache management CLI commands
    #[allow(dead_code)]
    pub fn has_valid(&self, anidb_id: u32) -> bool {
        self.get(anidb_id).is_some()
    }

    /// Insert or update a cache entry
    pub fn insert(&mut self, info: &AnimeInfo) {
        let entry = CacheEntry::from_anime_info(info);
        debug!("Caching anime {}", entry.anidb_id);
        self.data.entries.insert(entry.anidb_id, entry);
        self.dirty = true;
    }

    /// Remove expired entries from cache
    ///
    /// TODO(feature-61): Cache management CLI commands
    #[allow(dead_code)]
    pub fn prune_expired(&mut self) -> usize {
        let expiry_days = self.config.expiry_days;
        let before_count = self.data.entries.len();

        self.data
            .entries
            .retain(|_, entry| !entry.is_expired(expiry_days));

        let removed = before_count - self.data.entries.len();
        if removed > 0 {
            info!("Pruned {} expired cache entries", removed);
            self.dirty = true;
        }
        removed
    }

    /// Clear all cached entries
    ///
    /// TODO(feature-61): Cache management CLI commands
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.data.entries.clear();
        self.dirty = true;
    }

    /// Save cache to disk if modified
    pub fn save(&mut self) -> Result<(), CacheError> {
        if !self.dirty {
            debug!("Cache not modified, skipping save");
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.config.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temporary file first (atomic write)
        let temp_path = self.config.cache_path.with_extension("json.tmp");

        {
            let file = File::create(&temp_path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, &self.data)?;
        }

        // Rename temp file to actual cache file
        fs::rename(&temp_path, &self.config.cache_path)?;

        self.dirty = false;
        info!(
            "Saved cache with {} entries to {:?}",
            self.data.entries.len(),
            self.config.cache_path
        );
        Ok(())
    }

    /// Get number of cached entries
    ///
    /// TODO(feature-61): Cache management CLI commands
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.data.entries.len()
    }

    /// Check if cache is empty
    ///
    /// TODO(feature-61): Cache management CLI commands
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.data.entries.is_empty()
    }

    /// Get the cache file path
    ///
    /// TODO(feature-61): Cache management CLI commands
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.config.cache_path
    }
}

impl Drop for CacheStore {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            warn!("Failed to save cache on drop: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::tempdir;

    fn create_test_info(id: u32) -> AnimeInfo {
        AnimeInfo {
            anidb_id: id,
            title_main: format!("Test Anime {}", id),
            title_en: Some(format!("Test Anime {} EN", id)),
            release_year: Some(2020),
        }
    }

    fn create_expired_entry(id: u32) -> CacheEntry {
        CacheEntry {
            anidb_id: id,
            title_main: format!("Expired Anime {}", id),
            title_en: None,
            release_year: None,
            fetched_at: Utc::now() - Duration::days(60),
        }
    }

    #[test]
    fn test_cache_hit() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);

        let info = create_test_info(12345);
        cache.insert(&info);

        let retrieved = cache.get(12345);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title_main, "Test Anime 12345");
    }

    #[test]
    fn test_cache_miss() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let cache = CacheStore::load(config);

        assert!(cache.get(99999).is_none());
    }

    #[test]
    fn test_has_valid() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);

        assert!(!cache.has_valid(1));

        cache.insert(&create_test_info(1));
        assert!(cache.has_valid(1));
    }

    #[test]
    fn test_expired_entry_not_returned() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);

        // Insert expired entry directly
        cache.data.entries.insert(1, create_expired_entry(1));

        // Should return None for expired entry
        assert!(cache.get(1).is_none());
        assert!(!cache.has_valid(1));
    }

    #[test]
    fn test_prune_expired() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);

        // Insert fresh entry
        cache.insert(&create_test_info(1));

        // Insert expired entry directly
        cache.data.entries.insert(2, create_expired_entry(2));

        assert_eq!(cache.len(), 2);

        let removed = cache.prune_expired();

        assert_eq!(removed, 1);
        assert_eq!(cache.len(), 1);
        assert!(cache.get(1).is_some());
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn test_clear() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);

        cache.insert(&create_test_info(1));
        cache.insert(&create_test_info(2));

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_persistence() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);

        // Create and save cache
        {
            let mut cache = CacheStore::load(config.clone());
            cache.insert(&AnimeInfo {
                anidb_id: 12345,
                title_main: "Persisted".to_string(),
                title_en: None,
                release_year: None,
            });
            cache.save().unwrap();
        }

        // Load cache and verify
        {
            let cache = CacheStore::load(config);
            let retrieved = cache.get(12345);
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().title_main, "Persisted");
        }
    }

    #[test]
    fn test_corrupted_cache_handling() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join(".anidb2folder-cache.json");

        // Write corrupted JSON
        fs::write(&cache_path, "{ invalid json }").unwrap();

        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let cache = CacheStore::load(config);

        // Should start with empty cache
        assert!(cache.is_empty());
    }

    #[test]
    fn test_version_mismatch_handling() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join(".anidb2folder-cache.json");

        // Write cache with different version
        let old_cache = r#"{"version": "0.1", "entries": {}}"#;
        fs::write(&cache_path, old_cache).unwrap();

        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let cache = CacheStore::load(config);

        // Should start fresh due to version mismatch
        assert!(cache.is_empty());
    }

    #[test]
    fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config.clone());

        cache.insert(&create_test_info(1));
        cache.save().unwrap();

        // Verify no temp file left behind
        let temp_path = config.cache_path.with_extension("json.tmp");
        assert!(!temp_path.exists());
        assert!(config.cache_path.exists());
    }

    #[test]
    fn test_skip_save_when_not_dirty() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config.clone());

        // Save without modifications should succeed and not create file
        cache.save().unwrap();
        assert!(!config.cache_path.exists());

        // Insert and save should create file
        cache.insert(&create_test_info(1));
        cache.save().unwrap();
        assert!(config.cache_path.exists());

        // Another save without modifications should be a no-op
        cache.save().unwrap();
    }

    #[test]
    fn test_len_and_is_empty() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        cache.insert(&create_test_info(1));

        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);

        cache.insert(&create_test_info(2));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_update_existing_entry() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);

        cache.insert(&AnimeInfo {
            anidb_id: 1,
            title_main: "Original".to_string(),
            title_en: None,
            release_year: None,
        });

        cache.insert(&AnimeInfo {
            anidb_id: 1,
            title_main: "Updated".to_string(),
            title_en: Some("Updated EN".to_string()),
            release_year: Some(2021),
        });

        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(1).unwrap();
        assert_eq!(retrieved.title_main, "Updated");
        assert_eq!(retrieved.title_en, Some("Updated EN".to_string()));
    }
}
