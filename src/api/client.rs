use super::types::{AnimeInfo, ApiConfig, ApiError};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::blocking::Client;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const API_BASE_URL: &str = "http://api.anidb.net:9001/httpapi";
const PROTOCOL_VERSION: u32 = 1;

/// Rate limiter to ensure we don't exceed AniDB's request limits
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
                let wait_time = self.min_interval - elapsed;
                debug!("Rate limiting: waiting {:?}", wait_time);
                std::thread::sleep(wait_time);
            }
        }

        *last = Some(Instant::now());
    }
}

/// AniDB HTTP API client
pub struct AniDbClient {
    client: Client,
    config: ApiConfig,
    rate_limiter: RateLimiter,
}

impl AniDbClient {
    /// Create a new AniDB client with the given configuration
    pub fn new(config: ApiConfig) -> Result<Self, ApiError> {
        if !config.is_configured() {
            return Err(ApiError::NotConfigured);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .gzip(true)
            .user_agent(format!(
                "{}/{}",
                config.client_name, config.client_version
            ))
            .build()
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        let rate_limiter =
            RateLimiter::new(Duration::from_secs(config.min_request_interval_secs));

        Ok(Self {
            client,
            config,
            rate_limiter,
        })
    }

    /// Fetch anime information by AniDB ID with retry logic
    pub fn fetch_anime(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        let mut last_error = None;
        let mut delay = Duration::from_secs(1);

        for attempt in 1..=self.config.max_retries {
            info!(
                "Fetching anime {} (attempt {}/{})",
                anidb_id, attempt, self.config.max_retries
            );

            self.rate_limiter.wait_if_needed();

            match self.fetch_anime_internal(anidb_id) {
                Ok(info) => {
                    info!(
                        "Successfully fetched anime {}: {}",
                        anidb_id, info.title_main
                    );
                    return Ok(info);
                }
                Err(e) => {
                    warn!("Attempt {} failed: {}", attempt, e);

                    // Don't retry for certain errors
                    if matches!(
                        e,
                        ApiError::NotFound(_)
                            | ApiError::Banned(_)
                            | ApiError::NotConfigured
                            | ApiError::IncompleteData { .. }
                    ) {
                        return Err(e);
                    }

                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        debug!("Waiting {:?} before retry", delay);
                        std::thread::sleep(delay);
                        delay *= 2; // Exponential backoff
                    }
                }
            }
        }

        Err(last_error.unwrap_or(ApiError::MaxRetriesExceeded {
            attempts: self.config.max_retries,
        }))
    }

    fn fetch_anime_internal(&self, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        let url = format!(
            "{}?request=anime&client={}&clientver={}&protover={}&aid={}",
            API_BASE_URL,
            self.config.client_name,
            self.config.client_version,
            PROTOCOL_VERSION,
            anidb_id
        );

        debug!("Requesting: {}", url);

        let response = self.client.get(&url).send()?;
        let status = response.status();

        debug!("Response status: {}", status);

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ApiError::RateLimited);
        }

        let body = response.text()?;

        // Check for error responses
        if body.contains("<error>") {
            return self.parse_error_response(&body, anidb_id);
        }

        self.parse_anime_xml(anidb_id, &body)
    }

    fn parse_error_response(&self, body: &str, anidb_id: u32) -> Result<AnimeInfo, ApiError> {
        // Extract error message from XML
        if let Some(start) = body.find("<error>") {
            if let Some(end) = body.find("</error>") {
                let error_msg = &body[start + 7..end];
                let error_lower = error_msg.to_lowercase();

                if error_lower.contains("anime not found")
                    || error_lower.contains("no such anime")
                {
                    return Err(ApiError::NotFound(anidb_id));
                }

                if error_lower.contains("banned") || error_lower.contains("client") {
                    return Err(ApiError::Banned(error_msg.to_string()));
                }

                return Err(ApiError::ServerError(error_msg.to_string()));
            }
        }

        Err(ApiError::ServerError(body.to_string()))
    }

    fn parse_anime_xml(&self, anidb_id: u32, xml: &str) -> Result<AnimeInfo, ApiError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut title_main: Option<String> = None;
        let mut title_en: Option<String> = None;
        let mut release_year: Option<u16> = None;

        let mut buf = Vec::new();
        let mut in_titles = false;
        let mut in_startdate = false;
        let mut current_title_type: Option<String> = None;
        let mut current_title_lang: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        b"titles" => in_titles = true,
                        b"title" if in_titles => {
                            current_title_type = None;
                            current_title_lang = None;

                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"type" => {
                                        current_title_type = Some(
                                            String::from_utf8_lossy(&attr.value).to_string(),
                                        );
                                    }
                                    b"xml:lang" => {
                                        current_title_lang = Some(
                                            String::from_utf8_lossy(&attr.value).to_string(),
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                        b"startdate" => in_startdate = true,
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();

                    if in_startdate && !text.is_empty() {
                        // Parse year from startdate (format: YYYY-MM-DD or YYYY)
                        if let Some(year_str) = text.split('-').next() {
                            if let Ok(year) = year_str.parse::<u16>() {
                                release_year = Some(year);
                            }
                        }
                        in_startdate = false;
                    }

                    if in_titles {
                        if let (Some(ref t_type), Some(ref t_lang)) =
                            (&current_title_type, &current_title_lang)
                        {
                            // Main title (romaji)
                            if t_type == "main" {
                                title_main = Some(text.clone());
                            }
                            // Fallback: official romaji title if no main title
                            else if t_type == "official" && t_lang == "x-jat" && title_main.is_none()
                            {
                                title_main = Some(text.clone());
                            }
                            // English title
                            else if t_type == "official" && t_lang == "en" {
                                title_en = Some(text.clone());
                            }
                        }
                    }
                }
                Ok(Event::End(ref e)) => match e.name().as_ref() {
                    b"titles" => in_titles = false,
                    b"title" => {
                        current_title_type = None;
                        current_title_lang = None;
                    }
                    b"startdate" => in_startdate = false,
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(ApiError::ParseError(format!(
                        "XML parse error at position {}: {}",
                        reader.buffer_position(),
                        e
                    )));
                }
                _ => {}
            }
            buf.clear();
        }

        let title_main = title_main.ok_or_else(|| ApiError::IncompleteData {
            anidb_id,
            field: "main title".to_string(),
        })?;

        Ok(AnimeInfo {
            anidb_id,
            title_main,
            title_en,
            release_year,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ApiConfig {
        ApiConfig::new("testclient", 1)
    }

    #[test]
    fn test_client_requires_config() {
        let result = AniDbClient::new(ApiConfig::default());
        assert!(matches!(result, Err(ApiError::NotConfigured)));
    }

    #[test]
    fn test_client_creation() {
        let config = test_config();
        let client = AniDbClient::new(config);
        assert!(client.is_ok());
    }

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

        let config = test_config();
        let client = AniDbClient::new(config).unwrap();
        let result = client.parse_anime_xml(1, xml).unwrap();

        assert_eq!(result.anidb_id, 1);
        assert_eq!(result.title_main, "Cowboy Bebop");
        assert_eq!(result.title_en, Some("Cowboy Bebop".to_string()));
        assert_eq!(result.release_year, Some(1998));
    }

    #[test]
    fn test_parse_anime_xml_no_english() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <anime id="2">
            <titles>
                <title xml:lang="x-jat" type="main">Some Japanese Title</title>
            </titles>
        </anime>"#;

        let config = test_config();
        let client = AniDbClient::new(config).unwrap();
        let result = client.parse_anime_xml(2, xml).unwrap();

        assert_eq!(result.title_main, "Some Japanese Title");
        assert!(result.title_en.is_none());
        assert!(result.release_year.is_none());
    }

    #[test]
    fn test_parse_anime_xml_year_only() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <anime id="3">
            <titles>
                <title xml:lang="x-jat" type="main">Test Anime</title>
            </titles>
            <startdate>2020</startdate>
        </anime>"#;

        let config = test_config();
        let client = AniDbClient::new(config).unwrap();
        let result = client.parse_anime_xml(3, xml).unwrap();

        assert_eq!(result.release_year, Some(2020));
    }

    #[test]
    fn test_parse_anime_xml_no_main_title_uses_official() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <anime id="4">
            <titles>
                <title xml:lang="x-jat" type="official">Fallback Romaji Title</title>
                <title xml:lang="en" type="official">English Title</title>
            </titles>
        </anime>"#;

        let config = test_config();
        let client = AniDbClient::new(config).unwrap();
        let result = client.parse_anime_xml(4, xml).unwrap();

        assert_eq!(result.title_main, "Fallback Romaji Title");
        assert_eq!(result.title_en, Some("English Title".to_string()));
    }

    #[test]
    fn test_parse_anime_xml_missing_title_error() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <anime id="5">
            <titles>
            </titles>
        </anime>"#;

        let config = test_config();
        let client = AniDbClient::new(config).unwrap();
        let result = client.parse_anime_xml(5, xml);

        assert!(matches!(
            result,
            Err(ApiError::IncompleteData { anidb_id: 5, .. })
        ));
    }

    #[test]
    fn test_parse_error_response_not_found() {
        let body = "<error>Anime not found</error>";

        let config = test_config();
        let client = AniDbClient::new(config).unwrap();
        let result = client.parse_error_response(body, 99999);

        assert!(matches!(result, Err(ApiError::NotFound(99999))));
    }

    #[test]
    fn test_parse_error_response_banned() {
        let body = "<error>Banned: Client version outdated</error>";

        let config = test_config();
        let client = AniDbClient::new(config).unwrap();
        let result = client.parse_error_response(body, 1);

        assert!(matches!(result, Err(ApiError::Banned(_))));
    }

    #[test]
    fn test_parse_error_response_generic() {
        let body = "<error>Unknown error occurred</error>";

        let config = test_config();
        let client = AniDbClient::new(config).unwrap();
        let result = client.parse_error_response(body, 1);

        assert!(matches!(result, Err(ApiError::ServerError(_))));
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(Duration::from_millis(100));

        let start = Instant::now();
        limiter.wait_if_needed();
        limiter.wait_if_needed();
        let elapsed = start.elapsed();

        // Second call should have waited at least 100ms
        assert!(elapsed >= Duration::from_millis(100));
    }
}
