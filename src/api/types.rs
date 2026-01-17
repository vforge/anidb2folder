use thiserror::Error;

/// Anime information fetched from AniDB
#[derive(Debug, Clone)]
pub struct AnimeInfo {
    pub anidb_id: u32,
    pub title_main: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
}

/// API client configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub client_name: String,
    pub client_version: u32,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub min_request_interval_secs: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            client_name: String::new(),
            client_version: 1,
            timeout_secs: 30,
            max_retries: 3,
            min_request_interval_secs: 2,
        }
    }
}

impl ApiConfig {
    pub fn new(client_name: impl Into<String>, client_version: u32) -> Self {
        Self {
            client_name: client_name.into(),
            client_version,
            ..Default::default()
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.client_name.is_empty()
    }
}

/// Errors that can occur when interacting with the AniDB API
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Anime not found: {0}")]
    NotFound(u32),

    #[error("Rate limited by AniDB")]
    RateLimited,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("API returned error: {0}")]
    ServerError(String),

    #[error("Max retries exceeded after {attempts} attempts")]
    MaxRetriesExceeded { attempts: u32 },

    #[error("Client not configured: ANIDB_CLIENT and ANIDB_CLIENT_VERSION must be set")]
    NotConfigured,

    #[error("Banned by AniDB: {0}")]
    Banned(String),
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ApiError::Timeout
        } else {
            ApiError::NetworkError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anime_info_creation() {
        let info = AnimeInfo {
            anidb_id: 1,
            title_main: "Cowboy Bebop".to_string(),
            title_en: Some("Cowboy Bebop".to_string()),
            release_year: Some(1998),
        };

        assert_eq!(info.anidb_id, 1);
        assert_eq!(info.title_main, "Cowboy Bebop");
        assert_eq!(info.title_en, Some("Cowboy Bebop".to_string()));
        assert_eq!(info.release_year, Some(1998));
    }

    #[test]
    fn test_anime_info_optional_fields() {
        let info = AnimeInfo {
            anidb_id: 2,
            title_main: "Some Anime".to_string(),
            title_en: None,
            release_year: None,
        };

        assert!(info.title_en.is_none());
        assert!(info.release_year.is_none());
    }

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();

        assert!(config.client_name.is_empty());
        assert_eq!(config.client_version, 1);
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.min_request_interval_secs, 2);
    }

    #[test]
    fn test_api_config_new() {
        let config = ApiConfig::new("myclient", 2);

        assert_eq!(config.client_name, "myclient");
        assert_eq!(config.client_version, 2);
        assert!(!config.client_name.is_empty());
    }

    #[test]
    fn test_api_config_is_configured() {
        let unconfigured = ApiConfig::default();
        assert!(!unconfigured.is_configured());

        let configured = ApiConfig::new("myclient", 1);
        assert!(configured.is_configured());
    }

    #[test]
    fn test_api_error_display() {
        let err = ApiError::NotFound(12345);
        assert!(err.to_string().contains("12345"));

        let err = ApiError::RateLimited;
        assert!(err.to_string().contains("Rate limited"));

        let err = ApiError::MaxRetriesExceeded { attempts: 3 };
        assert!(err.to_string().contains("3 attempts"));
    }
}
