# 11 - Local Cache

## Summary

Implement a local JSON cache to store fetched anime metadata and minimize API calls.

## Dependencies

- **10-anidb-api-client** — Requires API types (`AnimeInfo`) and client for cache population

## Description

This feature implements a local caching system for anime metadata fetched from AniDB. The cache stores anime information in a JSON file to avoid redundant API calls on subsequent executions.

Key functionality:

- Store anime metadata with fetch timestamps
- Configurable cache expiration (default: 30 days)
- Automatic cache invalidation for expired entries
- Graceful handling of corrupted cache files
- Two storage location options (target directory or user home)

## Requirements

### Functional Requirements

1. Store and retrieve `AnimeInfo` entries by AniDB ID
2. Track fetch timestamp for each cached entry
3. Expire entries older than the configured duration
4. Support two storage locations:
   - Target directory: `.anidb2folder-cache.json`
   - User home: `~/.cache/anidb2folder/cache.json`
5. Handle corrupted cache gracefully (ignore and rebuild)
6. Provide methods to:
   - Get cached entry (if valid)
   - Insert/update entry
   - Check if entry exists and is valid
   - Clear expired entries
   - Clear entire cache

### Non-Functional Requirements

1. Use `serde_json` for JSON serialization
2. Atomic file writes (write to temp, then rename)
3. Human-readable JSON format (pretty-printed)
4. Include cache version for future migrations

## Implementation Guide

### Step 1: Add Dependencies

```toml
# Cargo.toml additions
[dependencies]
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
dirs = "5.0"
```

### Step 2: Define Cache Structures

```rust
// src/cache/mod.rs
mod store;
mod types;

pub use store::CacheStore;
pub use types::{CacheConfig, CacheEntry, CacheError};
```

```rust
// src/cache/types.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

use crate::api::AnimeInfo;

pub const CACHE_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub anidb_id: u32,
    pub title_jp: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
    pub fetched_at: DateTime<Utc>,
}

impl CacheEntry {
    pub fn from_anime_info(info: AnimeInfo) -> Self {
        Self {
            anidb_id: info.anidb_id,
            title_jp: info.title_jp,
            title_en: info.title_en,
            release_year: info.release_year,
            fetched_at: Utc::now(),
        }
    }
    
    pub fn to_anime_info(&self) -> AnimeInfo {
        AnimeInfo {
            anidb_id: self.anidb_id,
            title_jp: self.title_jp.clone(),
            title_en: self.title_en.clone(),
            release_year: self.release_year,
        }
    }
    
    pub fn is_expired(&self, expiry_days: u32) -> bool {
        let age = Utc::now().signed_duration_since(self.fetched_at);
        age.num_days() > expiry_days as i64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheFile {
    pub version: String,
    pub entries: std::collections::HashMap<u32, CacheEntry>,
}

impl Default for CacheFile {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION.to_string(),
            entries: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub expiry_days: u32,
    pub cache_path: PathBuf,
}

impl CacheConfig {
    pub fn for_target_dir(target: &std::path::Path, expiry_days: u32) -> Self {
        Self {
            expiry_days,
            cache_path: target.join(".anidb2folder-cache.json"),
        }
    }
    
    pub fn for_user_home(expiry_days: u32) -> Option<Self> {
        dirs::cache_dir().map(|cache_dir| Self {
            expiry_days,
            cache_path: cache_dir.join("anidb2folder").join("cache.json"),
        })
    }
}

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache file corrupted, will rebuild")]
    Corrupted,
    
    #[error("Failed to read cache: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Failed to serialize cache: {0}")]
    SerializeError(#[from] serde_json::Error),
    
    #[error("Cache version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
}
```

### Step 3: Implement Cache Store

```rust
// src/cache/store.rs
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use tracing::{debug, info, warn};

use crate::api::AnimeInfo;
use super::types::{CacheConfig, CacheEntry, CacheError, CacheFile, CACHE_VERSION};

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
                match e {
                    CacheError::ReadError(ref io_err) 
                        if io_err.kind() == std::io::ErrorKind::NotFound => {
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
        let cache: CacheFile = serde_json::from_reader(reader)
            .map_err(|_| CacheError::Corrupted)?;
        
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
    pub fn has_valid(&self, anidb_id: u32) -> bool {
        self.get(anidb_id).is_some()
    }
    
    /// Insert or update a cache entry
    pub fn insert(&mut self, info: AnimeInfo) {
        let entry = CacheEntry::from_anime_info(info);
        debug!("Caching anime {}", entry.anidb_id);
        self.data.entries.insert(entry.anidb_id, entry);
        self.dirty = true;
    }
    
    /// Remove expired entries from cache
    pub fn prune_expired(&mut self) -> usize {
        let expiry_days = self.config.expiry_days;
        let before_count = self.data.entries.len();
        
        self.data.entries.retain(|_, entry| !entry.is_expired(expiry_days));
        
        let removed = before_count - self.data.entries.len();
        if removed > 0 {
            info!("Pruned {} expired cache entries", removed);
            self.dirty = true;
        }
        removed
    }
    
    /// Clear all cached entries
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
        info!("Saved cache with {} entries", self.data.entries.len());
        Ok(())
    }
    
    /// Get number of cached entries
    pub fn len(&self) -> usize {
        self.data.entries.len()
    }
    
    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.data.entries.is_empty()
    }
}

impl Drop for CacheStore {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            warn!("Failed to save cache on drop: {}", e);
        }
    }
}
```

### Step 4: Create Cached API Client Wrapper

```rust
// src/api/cached_client.rs
use crate::api::{AniDbClient, AnimeInfo, ApiError};
use crate::cache::CacheStore;
use tracing::info;

pub struct CachedAniDbClient {
    api_client: AniDbClient,
    cache: CacheStore,
}

impl CachedAniDbClient {
    pub fn new(api_client: AniDbClient, cache: CacheStore) -> Self {
        Self { api_client, cache }
    }
    
    pub fn fetch_anime(&mut self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        // Check cache first
        if let Some(info) = self.cache.get(anidb_id) {
            return Ok(info);
        }
        
        // Fetch from API
        info!("Fetching anime {} from API", anidb_id);
        let info = self.api_client.fetch_anime(anidb_id)?;
        
        // Cache the result
        self.cache.insert(info.clone());
        
        Ok(info)
    }
    
    pub fn save_cache(&mut self) -> Result<(), crate::cache::CacheError> {
        self.cache.save()
    }
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::tempdir;
    
    fn create_test_entry(anidb_id: u32, days_ago: i64) -> CacheEntry {
        CacheEntry {
            anidb_id,
            title_jp: format!("Test Anime {}", anidb_id),
            title_en: Some(format!("Test Anime {} EN", anidb_id)),
            release_year: Some(2020),
            fetched_at: Utc::now() - Duration::days(days_ago),
        }
    }
    
    #[test]
    fn test_cache_hit() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);
        
        let info = AnimeInfo {
            anidb_id: 12345,
            title_jp: "Test".to_string(),
            title_en: None,
            release_year: None,
        };
        
        cache.insert(info.clone());
        
        let retrieved = cache.get(12345);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title_jp, "Test");
    }
    
    #[test]
    fn test_cache_miss() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let cache = CacheStore::load(config);
        
        assert!(cache.get(99999).is_none());
    }
    
    #[test]
    fn test_cache_expiration() {
        let entry = create_test_entry(1, 31); // 31 days old
        assert!(entry.is_expired(30));
        
        let entry = create_test_entry(2, 29); // 29 days old
        assert!(!entry.is_expired(30));
    }
    
    #[test]
    fn test_prune_expired() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config);
        
        // Insert fresh entry
        cache.data.entries.insert(1, create_test_entry(1, 5));
        // Insert expired entry
        cache.data.entries.insert(2, create_test_entry(2, 35));
        
        let removed = cache.prune_expired();
        
        assert_eq!(removed, 1);
        assert!(cache.get(1).is_some());
        assert!(cache.get(2).is_none());
    }
    
    #[test]
    fn test_cache_persistence() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        
        // Create and save cache
        {
            let mut cache = CacheStore::load(config.clone());
            cache.insert(AnimeInfo {
                anidb_id: 12345,
                title_jp: "Persisted".to_string(),
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
            assert_eq!(retrieved.unwrap().title_jp, "Persisted");
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
    fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let config = CacheConfig::for_target_dir(dir.path(), 30);
        let mut cache = CacheStore::load(config.clone());
        
        cache.insert(AnimeInfo {
            anidb_id: 1,
            title_jp: "Test".to_string(),
            title_en: None,
            release_year: None,
        });
        cache.save().unwrap();
        
        // Verify no temp file left behind
        let temp_path = config.cache_path.with_extension("json.tmp");
        assert!(!temp_path.exists());
        assert!(config.cache_path.exists());
    }
}
```

### Integration Tests

```rust
// tests/cache_integration_tests.rs

#[test]
fn test_cached_client_uses_cache() {
    // Verify API is only called once for repeated requests
}

#[test]
fn test_cache_survives_restart() {
    // Verify cache persists between program runs
}
```

## Notes

- The atomic write pattern prevents data loss if the program crashes during save
- `Drop` implementation ensures cache is saved when the store goes out of scope
- Cache version allows for future migrations when format changes
- Consider adding cache compression for large caches in the future
- The user home cache location requires the `dirs` crate for cross-platform paths
