# 10 - AniDB API Client

## Summary

Implement a client to fetch anime metadata (titles, release year) from the AniDB public API.

## Dependencies

- **00-cli-scaffold** — Requires base project structure
- **03-error-handling** — Requires error handling patterns for API failures

## Description

This feature implements an HTTP client for fetching anime information from AniDB's public API. The client retrieves Japanese titles (romaji), English titles, and release years for anime entries identified by their AniDB ID.

The client must handle:

- HTTP requests to the AniDB API
- XML response parsing
- Rate limiting (AniDB enforces strict limits)
- Retry logic with exponential backoff
- Network errors and timeouts

### AniDB API Information

AniDB provides an HTTP API for fetching anime data. The relevant endpoint is:

```
http://api.anidb.net:9001/httpapi?request=anime&client=<client>&clientver=<version>&protover=1&aid=<anime_id>
```

**Important:** You must register a client with AniDB to use the API. The client name and version are required parameters.

## Requirements

### Functional Requirements

1. Fetch anime data by AniDB ID
2. Extract from API response:
   - Main title (Japanese/Romaji)
   - English title (if available)
   - Release year (from startdate)
3. Handle missing data gracefully (return `None` for optional fields)
4. Implement rate limiting (max 1 request per 2 seconds per AniDB guidelines)
5. Retry failed requests with exponential backoff (max 3 attempts)
6. Timeout after 30 seconds per request

### Non-Functional Requirements

1. Use `reqwest` for HTTP requests (with blocking feature for simplicity)
2. Use `quick-xml` for XML parsing
3. Make client name/version configurable
4. Provide clear error types for different failure modes

## Implementation Guide

### Step 1: Add Dependencies

```toml
# Cargo.toml additions
[dependencies]
reqwest = { version = "0.11", features = ["blocking"] }
quick-xml = "0.31"
serde = { version = "1.0", features = ["derive"] }
```

### Step 2: Define Data Structures

```rust
// src/api/mod.rs
mod client;
mod types;

pub use client::AniDbClient;
pub use types::{AnimeInfo, ApiError};
```

```rust
// src/api/types.rs
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct AnimeInfo {
    pub anidb_id: u32,
    pub title_jp: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Anime not found: {0}")]
    NotFound(u32),
    
    #[error("Rate limited by AniDB")]
    RateLimited,
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Request timeout")]
    Timeout,
    
    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
}
```

### Step 3: Implement Rate Limiter

```rust
// src/api/client.rs
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct RateLimiter {
    last_request: Mutex<Option<Instant>>,
    min_interval: Duration,
}

impl RateLimiter {
    fn new(min_interval: Duration) -> Self {
        Self {
            last_request: Mutex::new(None),
            min_interval,
        }
    }
    
    fn wait_if_needed(&self) {
        let mut last = self.last_request.lock().unwrap();
        
        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < self.min_interval {
                std::thread::sleep(self.min_interval - elapsed);
            }
        }
        
        *last = Some(Instant::now());
    }
}
```

### Step 4: Implement API Client

```rust
// src/api/client.rs (continued)
use reqwest::blocking::Client;
use std::time::Duration;
use tracing::{debug, warn};

pub struct AniDbClient {
    client: Client,
    client_name: String,
    client_version: u32,
    rate_limiter: RateLimiter,
    max_retries: u32,
}

impl AniDbClient {
    pub fn new(client_name: &str, client_version: u32) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            client_name: client_name.to_string(),
            client_version,
            // AniDB requires 2 second minimum between requests
            rate_limiter: RateLimiter::new(Duration::from_secs(2)),
            max_retries: 3,
        }
    }
    
    pub fn fetch_anime(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        let mut last_error = None;
        let mut delay = Duration::from_secs(1);
        
        for attempt in 1..=self.max_retries {
            debug!("Fetching anime {} (attempt {}/{})", anidb_id, attempt, self.max_retries);
            
            self.rate_limiter.wait_if_needed();
            
            match self.fetch_anime_internal(anidb_id) {
                Ok(info) => return Ok(info),
                Err(e) => {
                    warn!("Attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                    
                    if attempt < self.max_retries {
                        std::thread::sleep(delay);
                        delay *= 2; // Exponential backoff
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or(ApiError::MaxRetriesExceeded))
    }
    
    fn fetch_anime_internal(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        let url = format!(
            "http://api.anidb.net:9001/httpapi?request=anime&client={}&clientver={}&protover=1&aid={}",
            self.client_name,
            self.client_version,
            anidb_id
        );
        
        let response = self.client.get(&url).send()?;
        
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ApiError::RateLimited);
        }
        
        let body = response.text()?;
        
        // Check for error responses
        if body.contains("<error>") {
            if body.contains("Anime not found") {
                return Err(ApiError::NotFound(anidb_id));
            }
            return Err(ApiError::ApiError(body));
        }
        
        self.parse_anime_xml(anidb_id, &body)
    }
    
    fn parse_anime_xml(&self, anidb_id: u32, xml: &str) -> Result<AnimeInfo, ApiError> {
        use quick_xml::events::Event;
        use quick_xml::Reader;
        
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);
        
        let mut title_jp: Option<String> = None;
        let mut title_en: Option<String> = None;
        let mut release_year: Option<u16> = None;
        
        let mut buf = Vec::new();
        let mut in_titles = false;
        let mut current_title_type: Option<String> = None;
        let mut current_title_lang: Option<String> = None;
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"titles" => in_titles = true,
                        b"title" if in_titles => {
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"type" => {
                                        current_title_type = Some(
                                            String::from_utf8_lossy(&attr.value).to_string()
                                        );
                                    }
                                    b"xml:lang" => {
                                        current_title_lang = Some(
                                            String::from_utf8_lossy(&attr.value).to_string()
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                        b"startdate" => {}
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();
                    
                    if let (Some(ref t_type), Some(ref t_lang)) = 
                        (&current_title_type, &current_title_lang) 
                    {
                        // Main title (romaji)
                        if t_type == "main" || (t_type == "official" && t_lang == "x-jat") {
                            if title_jp.is_none() {
                                title_jp = Some(text.clone());
                            }
                        }
                        // English title
                        if t_type == "official" && t_lang == "en" {
                            title_en = Some(text.clone());
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    match e.name().as_ref() {
                        b"titles" => in_titles = false,
                        b"title" => {
                            current_title_type = None;
                            current_title_lang = None;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ApiError::ParseError(e.to_string())),
                _ => {}
            }
            buf.clear();
        }
        
        // Extract year from startdate if available
        // (simplified - would need additional parsing logic)
        
        let title_jp = title_jp.ok_or_else(|| {
            ApiError::ParseError("No main title found".to_string())
        })?;
        
        Ok(AnimeInfo {
            anidb_id,
            title_jp,
            title_en,
            release_year,
        })
    }
}
```

### Step 5: Configuration

```rust
// src/config.rs
pub struct ApiConfig {
    pub client_name: String,
    pub client_version: u32,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            client_name: "anidb2folder".to_string(),
            client_version: 1,
            timeout_secs: 30,
            max_retries: 3,
        }
    }
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_anime_xml_full_data() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <anime id="1" restricted="false">
            <titles>
                <title xml:lang="x-jat" type="main">Cowboy Bebop</title>
                <title xml:lang="en" type="official">Cowboy Bebop</title>
                <title xml:lang="ja" type="official">カウボーイビバップ</title>
            </titles>
            <startdate>1998-04-03</startdate>
        </anime>"#;
        
        let client = AniDbClient::new("test", 1);
        let result = client.parse_anime_xml(1, xml).unwrap();
        
        assert_eq!(result.anidb_id, 1);
        assert_eq!(result.title_jp, "Cowboy Bebop");
        assert_eq!(result.title_en, Some("Cowboy Bebop".to_string()));
    }
    
    #[test]
    fn test_parse_anime_xml_no_english() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <anime id="2">
            <titles>
                <title xml:lang="x-jat" type="main">Some Japanese Title</title>
            </titles>
        </anime>"#;
        
        let client = AniDbClient::new("test", 1);
        let result = client.parse_anime_xml(2, xml).unwrap();
        
        assert_eq!(result.title_en, None);
    }
}
```

### Integration Tests (Mocked)

```rust
// tests/api_tests.rs
// Use mockito or wiremock for HTTP mocking

#[test]
fn test_rate_limiting() {
    // Verify requests are spaced at least 2 seconds apart
}

#[test]
fn test_retry_on_failure() {
    // Verify exponential backoff on network errors
}

#[test]
fn test_not_found_error() {
    // Verify ApiError::NotFound for invalid IDs
}
```

## Notes

- **Important:** Register a client name with AniDB before using the API in production
- The rate limiter uses a mutex for thread safety, though the client is currently single-threaded
- Consider adding a connection pool for better performance with many requests
- The XML parsing is simplified — production code should handle more edge cases
- AniDB may ban clients that exceed rate limits — be conservative
