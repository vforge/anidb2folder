# 02 - Verbose Mode

## Summary

Provide detailed logging output for debugging and transparency.

## Dependencies

- **00-cli-scaffold** — Requires CLI flag parsing and logging infrastructure

## Description

This feature implements a verbose output mode that provides detailed logging information during execution. When enabled with `--verbose` or `-v`, the tool outputs additional information about each operation, API calls, cache hits/misses, and internal decisions.

Verbose mode is useful for:

- Debugging issues
- Understanding tool behavior
- Verifying operations before committing
- Troubleshooting API or cache problems

## Requirements

### Functional Requirements

1. Enable with `--verbose` or `-v` command-line flag
2. Log the following additional information:
   - Directory scan results
   - Format detection for each directory
   - Validation decisions
   - API requests and responses (summary)
   - Cache hits and misses
   - Name construction steps
   - Character sanitization changes
   - Truncation decisions
3. Use structured logging with log levels
4. Support multiple verbosity levels (optional: `-vv`, `-vvv`)

### Non-Functional Requirements

1. Use `tracing` crate for structured logging
2. Output to stderr (keep stdout clean for results)
3. Include timestamps in verbose output
4. Respect `NO_COLOR` environment variable

## Implementation Guide

### Step 1: Configure Logging Levels

```rust
// src/logging.rs
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    EnvFilter,
};

pub fn init_logging(verbosity: u8) {
    let level = match verbosity {
        0 => Level::WARN,   // Default: only warnings and errors
        1 => Level::INFO,   // -v: informational messages
        2 => Level::DEBUG,  // -vv: debug information
        _ => Level::TRACE,  // -vvv: trace everything
    };
    
    let filter = EnvFilter::from_default_env()
        .add_directive(level.into());
    
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(verbosity >= 2)        // Show module targets at debug+
        .with_thread_ids(verbosity >= 3)    // Show thread IDs at trace
        .with_file(verbosity >= 3)          // Show file/line at trace
        .with_line_number(verbosity >= 3)
        .with_span_events(if verbosity >= 2 { 
            FmtSpan::ENTER | FmtSpan::EXIT 
        } else { 
            FmtSpan::NONE 
        })
        .with_writer(std::io::stderr)       // Log to stderr
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
}
```

### Step 2: Update CLI for Verbosity Counting

```rust
// src/cli.rs (updated)
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "anidb2folder")]
pub struct Args {
    // ... other args ...
    
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}
```

### Step 3: Add Logging Throughout Codebase

```rust
// Example: src/scanner.rs with verbose logging
use tracing::{debug, info, trace, warn, instrument};

#[instrument(level = "debug", skip(target))]
pub fn scan_directory(target: &Path) -> Result<Vec<DirectoryEntry>, ScannerError> {
    info!(path = ?target, "Scanning directory");
    
    // ... validation ...
    
    let mut entries = Vec::new();
    
    for entry in fs::read_dir(target)? {
        let entry = entry?;
        let path = entry.path();
        
        trace!(entry = ?path, "Examining entry");
        
        if !path.is_dir() {
            debug!(path = ?path, "Skipping non-directory");
            continue;
        }
        
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        if name.starts_with('.') {
            debug!(name = %name, "Skipping hidden directory");
            continue;
        }
        
        debug!(name = %name, "Found directory");
        entries.push(DirectoryEntry::new(name, path));
    }
    
    info!(count = entries.len(), "Scan complete");
    
    Ok(entries)
}
```

```rust
// Example: src/api/client.rs with verbose logging
use tracing::{debug, info, warn, instrument, Span};

impl AniDbClient {
    #[instrument(level = "debug", skip(self))]
    pub fn fetch_anime(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        debug!(id = anidb_id, "Fetching anime info");
        
        self.rate_limiter.wait_if_needed();
        debug!("Rate limiter passed");
        
        let url = self.build_url(anidb_id);
        debug!(url = %url, "Making API request");
        
        let response = self.client.get(&url).send()?;
        debug!(status = %response.status(), "Received response");
        
        // ... parse response ...
        
        info!(
            id = anidb_id,
            title = %info.title_jp,
            "Successfully fetched anime"
        );
        
        Ok(info)
    }
}
```

```rust
// Example: src/cache/store.rs with verbose logging
use tracing::{debug, info, trace};

impl CacheStore {
    pub fn get(&self, anidb_id: u32) -> Option<AnimeInfo> {
        match self.data.entries.get(&anidb_id) {
            Some(entry) => {
                if entry.is_expired(self.config.expiry_days) {
                    debug!(id = anidb_id, age_days = ?entry.age_days(), "Cache miss (expired)");
                    None
                } else {
                    debug!(id = anidb_id, "Cache hit");
                    trace!(entry = ?entry, "Cached data");
                    Some(entry.to_anime_info())
                }
            }
            None => {
                debug!(id = anidb_id, "Cache miss (not found)");
                None
            }
        }
    }
}
```

```rust
// Example: src/rename/name_builder.rs with verbose logging
use tracing::{debug, trace, warn};

pub fn build_human_readable_name(
    series_tag: Option<&str>,
    info: &AnimeInfo,
    config: &NameBuilderConfig,
) -> NameBuildResult {
    debug!(
        anidb_id = info.anidb_id,
        series_tag = ?series_tag,
        "Building human-readable name"
    );
    
    trace!(
        title_jp = %info.title_jp,
        title_en = ?info.title_en,
        year = ?info.release_year,
        "Input data"
    );
    
    // ... name building ...
    
    if result.truncated {
        warn!(
            anidb_id = info.anidb_id,
            original_len = original.len(),
            truncated_len = result.name.len(),
            "Name was truncated"
        );
    }
    
    debug!(result = %result.name, "Built name");
    
    result
}
```

### Step 4: Update Main

```rust
// src/main.rs
mod logging;

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging based on verbosity
    logging::init_logging(args.verbose);
    
    // Rest of main...
}
```

## Example Output

### Normal Mode (no -v)

```
Successfully renamed 5 directories
History saved to: anidb2folder-history-20260115-103045.json
```

### Verbose Mode (-v)

```
2026-01-15T10:30:45Z INFO  anidb2folder: Starting anidb2folder v0.1.0
2026-01-15T10:30:45Z INFO  scanner: Scanning directory path="/home/user/anime"
2026-01-15T10:30:45Z INFO  scanner: Scan complete count=5
2026-01-15T10:30:45Z INFO  validator: Validating 5 directories
2026-01-15T10:30:45Z INFO  validator: All directories in AniDB format
2026-01-15T10:30:45Z INFO  api: Fetching anime id=12345
2026-01-15T10:30:46Z INFO  cache: Cache miss id=12345
2026-01-15T10:30:47Z INFO  api: Successfully fetched anime id=12345 title="Naruto"
2026-01-15T10:30:47Z INFO  rename: Renaming 12345 -> Naruto (2002) [anidb-12345]
...
Successfully renamed 5 directories
```

### Debug Mode (-vv)

```
2026-01-15T10:30:45Z DEBUG scanner: Examining entry entry="/home/user/anime/12345"
2026-01-15T10:30:45Z DEBUG scanner: Found directory name="12345"
2026-01-15T10:30:45Z DEBUG parser: Parsing directory name="12345"
2026-01-15T10:30:45Z DEBUG parser: Matched AniDB format series_tag=None id=12345
2026-01-15T10:30:45Z DEBUG api: Making API request url="http://api.anidb.net:9001/..."
2026-01-15T10:30:46Z DEBUG api: Received response status=200
2026-01-15T10:30:46Z DEBUG cache: Storing in cache id=12345
2026-01-15T10:30:46Z DEBUG sanitizer: Sanitizing name input="Naruto"
2026-01-15T10:30:46Z DEBUG sanitizer: No changes needed
...
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_logging_level_mapping() {
        // 0 -> WARN
        // 1 -> INFO
        // 2 -> DEBUG
        // 3+ -> TRACE
    }
}
```

### Integration Tests

```rust
#[test]
fn test_verbose_flag() {
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.arg("-v")
        .arg("/tmp/test")
        .assert()
        .stderr(predicate::str::contains("INFO"));
}

#[test]
fn test_very_verbose_flag() {
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.arg("-vv")
        .arg("/tmp/test")
        .assert()
        .stderr(predicate::str::contains("DEBUG"));
}
```

## Notes

- All logging goes to stderr to keep stdout clean for scripting
- Use `#[instrument]` attribute for automatic function tracing
- Consider adding JSON log output format for log aggregation
- Respect `NO_COLOR` and `RUST_LOG` environment variables
- Log levels allow filtering without changing code
