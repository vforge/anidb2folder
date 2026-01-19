# 64 - Web Data Source

## Summary

Add web scraping as an alternative data source option via `--source web` flag.

## Dependencies

- **10-anidb-api-client** — Extends the data fetching with alternative source
- **11-local-cache** — Cache entries track their data source

## Description

AniDB data can be fetched from two sources:
1. **HTTP API** — Fast (2s rate limit), requires registered client credentials
2. **Web scraping** — Slow (30s rate limit), no credentials needed, public pages

This feature adds web scraping as an explicit alternative data source that users can choose via a CLI flag. This is useful when:
- User doesn't have API credentials registered
- API is temporarily unavailable or banned
- User prefers not to use the API for any reason

### Current Behavior

Data is always fetched from the HTTP API. No alternative.

### Proposed Behavior

User can choose data source via `--source` flag:
```bash
# Default: use HTTP API
anidb2folder /path/to/anime

# Explicit: use web scraping
anidb2folder --source web /path/to/anime
```

Web source characteristics:
1. Rate limit: 1 request per 30 seconds (conservative)
2. No API credentials required
3. Cache entries marked with source for later validation
4. Slower but functional alternative

## Requirements

### Functional Requirements

1. Add `--source <api|web>` CLI flag (default: `api`)
2. Scrape anime data from `https://anidb.net/anime/{id}` when `--source web`
3. Enforce 30-second minimum interval between web requests
4. Extract from HTML:
   - Main title (romaji)
   - English title (if available)
   - Year (from start date)
5. Store data source in cache entries (`source: "api"` or `source: "web"`)
6. API can later validate/refresh web-sourced cache entries
7. Log clearly which source is being used

### Non-Functional Requirements

1. Respect AniDB's server resources — strict 30s rate limiting for web
2. No additional dependencies if possible (use existing reqwest)
3. Web scraping is opt-in, not automatic

## Implementation Guide

### Step 1: Add DataSource enum and CLI flag

```rust
// In src/cli.rs or args
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum DataSource {
    #[default]
    Api,
    Web,
}

// In CLI args
#[arg(long, default_value = "api")]
pub source: DataSource,
```

### Step 2: Add WebScraper struct

Create `src/api/web_scraper.rs`:

```rust
use super::types::{AnimeInfo, ApiError};
use reqwest::blocking::Client;
use std::time::{Duration, Instant};
use std::sync::Mutex;
use tracing::{debug, info, warn};

const WEB_BASE_URL: &str = "https://anidb.net/anime";
const WEB_MIN_INTERVAL: Duration = Duration::from_secs(30);

pub struct WebScraper {
    client: Client,
    last_request: Mutex<Option<Instant>>,
}

impl WebScraper {
    pub fn new() -> Result<Self, ApiError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (compatible; anidb2folder)")
            .build()
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        Ok(Self {
            client,
            last_request: Mutex::new(None),
        })
    }

    pub fn fetch_anime(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        self.wait_for_rate_limit();

        let url = format!("{}/{}", WEB_BASE_URL, anidb_id);
        info!("Fetching from web: {}", url);

        let response = self.client.get(&url).send()?;
        let html = response.text()?;

        self.parse_html(anidb_id, &html)
    }

    fn wait_for_rate_limit(&self) {
        let mut last = self.last_request.lock().unwrap();
        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < WEB_MIN_INTERVAL {
                let wait = WEB_MIN_INTERVAL - elapsed;
                info!("Web source rate limit: waiting {:?}", wait);
                std::thread::sleep(wait);
            }
        }
        *last = Some(Instant::now());
    }

    fn parse_html(&self, anidb_id: u32, html: &str) -> Result<AnimeInfo, ApiError> {
        // Extract main title from <h1 class="anime">...</h1>
        let title_main = self.extract_main_title(html)
            .ok_or_else(|| ApiError::IncompleteData {
                anidb_id,
                field: "main title".to_string(),
            })?;

        // Extract English title from data table (optional)
        let title_en = self.extract_english_title(html);

        // Extract year from "Year" or "Start Date" field
        let release_year = self.extract_year(html);

        Ok(AnimeInfo {
            anidb_id,
            title_main,
            title_en,
            release_year,
            source: DataSource::Web,  // Mark data source
        })
    }

    fn extract_main_title(&self, html: &str) -> Option<String> {
        // Look for: <h1 class="anime">Title</h1>
        // Simple regex-free parsing
        let start_marker = "<h1 class=\"anime\">";
        let start = html.find(start_marker)? + start_marker.len();
        let end = html[start..].find("</h1>")? + start;
        let title = html[start..end].trim();

        // Decode HTML entities
        Some(html_decode(title))
    }

    fn extract_english_title(&self, html: &str) -> Option<String> {
        // Look for English title in info table
        // Pattern: <span class="tagname">Main Title</span>...<span class="value">English Title</span>
        // This varies by page structure, may need refinement
        None // TODO: implement based on actual page structure
    }

    fn extract_year(&self, html: &str) -> Option<u16> {
        // Look for year in format "dd.mm.yyyy" or just "yyyy"
        // Usually in the "Year" or "Start Date" field
        None // TODO: implement based on actual page structure
    }
}

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
     .replace("&lt;", "<")
     .replace("&gt;", ">")
     .replace("&quot;", "\"")
     .replace("&#39;", "'")
}
```

### Step 3: Create DataFetcher trait and implementations

```rust
// In api/mod.rs - unified interface for data sources

pub trait DataFetcher {
    fn fetch_anime(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError>;
}

impl DataFetcher for AniDbClient {
    fn fetch_anime(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        let mut info = self.fetch_anime_via_api(anidb_id)?;
        info.source = DataSource::Api;
        Ok(info)
    }
}

impl DataFetcher for WebScraper {
    fn fetch_anime(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        // Already sets source = Web in parse_html
        self.fetch_anime_impl(anidb_id)
    }
}

// In main.rs - choose fetcher based on CLI flag
let fetcher: Box<dyn DataFetcher> = match args.source {
    DataSource::Api => Box::new(AniDbClient::new(config)?),
    DataSource::Web => {
        info!("Using web scraping (30s rate limit)");
        Box::new(WebScraper::new()?)
    }
};
```

### Step 4: Add module and exports

```rust
// In api/mod.rs
mod web_scraper;
pub use web_scraper::WebScraper;
```

### Step 5: Update AnimeInfo struct

```rust
// In api/types.rs
pub struct AnimeInfo {
    pub anidb_id: u32,
    pub title_main: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
    pub source: DataSource,  // Track where data came from
}
```

### Step 6: Update cache entry format

```rust
// In cache/types.rs
#[derive(Serialize, Deserialize)]
pub struct CacheEntry {
    pub anidb_id: u32,
    pub title_main: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
    pub source: String,      // "api" or "web"
    pub cached_at: String,   // ISO 8601 timestamp
}

// When caching:
let entry = CacheEntry {
    anidb_id: info.anidb_id,
    title_main: info.title_main.clone(),
    title_en: info.title_en.clone(),
    release_year: info.release_year,
    source: match info.source {
        DataSource::Api => "api".to_string(),
        DataSource::Web => "web".to_string(),
    },
    cached_at: chrono::Utc::now().to_rfc3339(),
};
```

This allows the API to later validate web-sourced entries by checking the `source` field and potentially refreshing them with API data when available.

## Test Cases

### Unit Tests

1. **test_web_scraper_rate_limit** — Enforces 30-second interval
2. **test_parse_html_main_title** — Extracts title from HTML
3. **test_parse_html_missing_title** — Returns error when title not found
4. **test_cache_entry_stores_source** — Cache entry includes source field
5. **test_data_source_cli_flag** — CLI parses --source correctly

### Integration Tests

1. **test_web_source_produces_valid_names** — Directory names are correct with web source
2. **test_cache_entry_source_field** — Cached entries have correct source value
3. **test_web_source_rate_limit_enforced** — 30s delays between requests

### Manual Testing

1. Test with known anime ID via web: `https://anidb.net/anime/1`
2. Verify extracted data matches API data
3. Test rate limiting: `anidb2folder --source web -v /path` (should see 30s delays)
4. Verify cache files contain `"source": "web"` field

## Notes

- **Rate limit is critical** — 30 seconds is conservative to respect AniDB servers
- **HTML structure may change** — Web scraping is fragile, may need updates
- **No API credentials needed** — Web pages are public, useful for users without registered clients
- **Cache tracks source** — Allows future validation/refresh of web-sourced data with API
- **Future enhancement** — Could add `--refresh-web-cache` to re-fetch web entries via API

## Design Decisions

1. **Explicit opt-in** — User must specify `--source web`; not automatic fallback
2. **Conservative rate limit** — 30 seconds between requests (slower than API's 2s)
3. **Cache source tracking** — Entries store whether data came from API or web
4. **No filename marker** — Source is tracked in cache, not in directory names
