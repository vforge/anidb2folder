# 61 - Cache Management CLI

## Summary

Add CLI commands for viewing, clearing, and pruning the local cache.

## Dependencies

- **11-local-cache** — Requires cache functionality to be implemented

## Description

This feature exposes cache management functionality through CLI flags. Users can view cache statistics, clear all cached entries, or prune only expired entries without performing any rename operations.

The cache stores anime metadata fetched from AniDB API to avoid repeated API calls. Over time, the cache may grow or contain stale entries.

## Requirements

### Functional Requirements

1. `--cache-info <DIR>` - Display cache information:
   - Cache file path
   - Total number of entries
   - Number of expired entries
   - Cache file size

2. `--cache-clear <DIR>` - Clear all cache entries:
   - Remove all cached data
   - Display count of removed entries

3. `--cache-prune <DIR>` - Prune expired entries:
   - Remove only expired entries (based on `--cache-expiry` setting)
   - Keep valid entries
   - Display count of removed entries

### Non-Functional Requirements

1. Cache commands are standalone (don't trigger rename operations)
2. Clear error messages if cache doesn't exist
3. Respect `--cache-expiry` setting for prune operation

## Implementation Guide

### Step 1: Update CLI

```rust
// src/cli.rs
#[derive(Parser, Debug)]
pub struct Args {
    // ... existing args ...

    /// Show cache information for a directory
    #[arg(long, value_name = "DIR")]
    pub cache_info: Option<PathBuf>,

    /// Clear all cached entries for a directory
    #[arg(long, value_name = "DIR")]
    pub cache_clear: Option<PathBuf>,

    /// Remove expired cache entries for a directory
    #[arg(long, value_name = "DIR")]
    pub cache_prune: Option<PathBuf>,
}
```

### Step 2: Add expired_count method to CacheStore

```rust
// src/cache/store.rs
pub fn expired_count(&self) -> usize {
    self.data.entries.values()
        .filter(|e| e.is_expired(self.config.expiry_days))
        .count()
}
```

### Step 3: Handle commands in main.rs

```rust
// Handle cache commands before normal operation
if let Some(dir) = &args.cache_info {
    return handle_cache_info(dir, args.cache_expiry, ui);
}

if let Some(dir) = &args.cache_clear {
    return handle_cache_clear(dir, args.cache_expiry, ui);
}

if let Some(dir) = &args.cache_prune {
    return handle_cache_prune(dir, args.cache_expiry, ui);
}
```

## Test Cases

### Unit Tests

1. Test cache info displays correct statistics
2. Test cache clear removes all entries
3. Test cache prune only removes expired entries

### Integration Tests

1. Test `--cache-info` with existing cache
2. Test `--cache-info` with no cache (should show "no cache found")
3. Test `--cache-clear` removes entries
4. Test `--cache-prune` only removes expired

## Notes

- Cache is stored per-directory at `<target>/.anidb2folder-cache.json`
- The `--cache-expiry` flag affects which entries are considered expired
- Consider adding `--global-cache` in future for centralized caching
