# 64 - Web Fallback on API Ban

## Summary

Fall back to scraping AniDB web pages when the HTTP API returns a ban/rate-limit response.

## Dependencies

- **10-anidb-api-client** — Extends the API client with fallback mechanism

## Description

AniDB's HTTP API can temporarily ban clients that exceed rate limits. When this happens, the API returns HTTP 500 with an XML response containing "banned". Currently, this causes the tool to fail completely.

This feature adds a fallback mechanism that switches to scraping the public AniDB web pages when the API is banned. Web scraping requires a much slower request rate (1 request per 15 seconds) but allows the tool to continue functioning.

### Current Behavior

When API is banned:
```
✗ Failed to fetch data for anime ID 12345:
  Banned by AniDB: <error message>
```

Tool stops processing remaining directories.

### Proposed Behavior

When API is banned:
1. Log warning about API ban and inform user about fallback mode
2. Automatically switch to web scraping mode (no opt-in required)
3. Increase rate limit to 15 seconds between requests
4. Continue processing with slower fallback
5. Mark directory names with `[web]` suffix to indicate fallback data source
6. If web scraping fails, abort (same as any other error)

## Requirements

### Functional Requirements

1. Detect API ban response (HTTP 500 + XML containing "banned")
2. Switch to web scraping mode automatically (no opt-in flag required)
3. Scrape anime data from `https://anidb.net/anime/{id}`
4. Enforce 15-second minimum interval between web requests
5. Extract from HTML:
   - Main title (romaji)
   - English title (if available)
   - Year (from start date)
6. Log clearly when switching to fallback mode (inform user once)
7. Mark directory names with `[web]` suffix when data came from web scraping
8. If web scraping fails for an anime, abort with error (same as API failures)
9. Continue processing remaining directories after ban (don't abort on ban itself)

### Non-Functional Requirements

1. Respect AniDB's server resources — strict rate limiting
2. Graceful degradation — slower but functional
3. No additional dependencies if possible (use existing reqwest)

## Implementation Guide

### Step 1: Add WebScraper struct

Create `src/api/web_scraper.rs`:

```rust
use super::types::{AnimeInfo, ApiError};
use reqwest::blocking::Client;
use std::time::{Duration, Instant};
use std::sync::Mutex;
use tracing::{debug, info, warn};

const WEB_BASE_URL: &str = "https://anidb.net/anime";
const WEB_MIN_INTERVAL: Duration = Duration::from_secs(15);

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
        info!("Fetching from web (fallback): {}", url);

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
                warn!("Web fallback rate limit: waiting {:?}", wait);
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
            from_web_fallback: true,  // Mark as web-sourced data
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

### Step 2: Update AniDbClient to use fallback

```rust
// In api/client.rs

pub struct AniDbClient {
    client: Client,
    config: ApiConfig,
    rate_limiter: RateLimiter,
    web_fallback: Option<WebScraper>,
    use_web_fallback: bool,
}

impl AniDbClient {
    pub fn fetch_anime(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        // If already in fallback mode, use web scraper
        if self.use_web_fallback {
            if let Some(ref scraper) = self.web_fallback {
                return scraper.fetch_anime(anidb_id);
            }
        }

        // Try API first
        match self.fetch_anime_via_api(anidb_id) {
            Ok(info) => Ok(info),
            Err(ApiError::Banned(msg)) => {
                warn!("API banned: {}. Switching to web fallback.", msg);
                self.enable_web_fallback();

                if let Some(ref scraper) = self.web_fallback {
                    scraper.fetch_anime(anidb_id)
                } else {
                    Err(ApiError::Banned(msg))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn enable_web_fallback(&mut self) {
        if self.web_fallback.is_none() {
            match WebScraper::new() {
                Ok(scraper) => {
                    self.web_fallback = Some(scraper);
                    self.use_web_fallback = true;
                }
                Err(e) => {
                    warn!("Failed to create web fallback: {}", e);
                }
            }
        }
        self.use_web_fallback = true;
    }
}
```

### Step 3: Add module and exports

```rust
// In api/mod.rs
mod web_scraper;
pub use web_scraper::WebScraper;
```

### Step 4: Update AnimeInfo struct

```rust
// In api/types.rs
pub struct AnimeInfo {
    pub anidb_id: u32,
    pub title_main: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
    pub from_web_fallback: bool,  // New field
}
```

### Step 5: Update name builder for [web] marker

```rust
// In name_builder.rs - when building the readable name
fn build_readable_name(&self, info: &AnimeInfo) -> String {
    let mut name = // ... existing logic ...

    // Add [web] marker if data came from web fallback
    if info.from_web_fallback {
        name.push_str(" [web]");
    }

    name
}
```

## Test Cases

### Unit Tests

1. **test_detect_ban_response** — Correctly identifies ban in API response
2. **test_web_scraper_rate_limit** — Enforces 15-second interval
3. **test_parse_html_main_title** — Extracts title from HTML
4. **test_parse_html_missing_title** — Returns error when title not found
5. **test_fallback_triggered_on_ban** — Switches to web on ban

### Integration Tests

1. **test_continues_after_ban** — Tool processes remaining dirs after ban
2. **test_web_fallback_produces_valid_names** — Directory names are correct
3. **test_web_fallback_adds_marker** — Directory names include `[web]` suffix
4. **test_web_failure_aborts** — Tool aborts when web scraping fails

### Manual Testing

1. Test with known anime ID via web: `https://anidb.net/anime/1`
2. Verify extracted data matches API data
3. Test rate limiting (should see 15s delays in verbose mode)

## Notes

- **Rate limit is critical** — 15 seconds is conservative, could potentially be 10s
- **HTML structure may change** — Web scraping is fragile, may need updates
- **No API credentials needed** — Web pages are public
- **Consider caching aggressively** — Minimize web requests
- **Future enhancement** — Could add `--web-only` flag to force web mode
- **Future enhancement** — Could periodically retry API to see if ban lifted

## Design Decisions

1. **Automatic fallback** — No opt-in flag required; web fallback activates automatically on API ban
2. **No special progress indicator** — User is informed once when fallback activates, no ongoing progress changes
3. **Abort on web failure** — If web scraping fails for an anime, abort with error (same behavior as API failures)
4. **Filename marker** — Directories renamed using web data include `[web]` suffix to indicate data source
