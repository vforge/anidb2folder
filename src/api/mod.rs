mod client;
mod types;

pub use client::AniDbClient;
pub use types::{AnimeInfo, ApiConfig, ApiError};

use std::env;

/// Environment variable names for AniDB client configuration
pub const ENV_ANIDB_CLIENT: &str = "ANIDB_CLIENT";
pub const ENV_ANIDB_CLIENT_VERSION: &str = "ANIDB_CLIENT_VERSION";

/// Load API configuration from environment variables
///
/// Required environment variables:
/// - `ANIDB_CLIENT`: Registered client name (lowercase)
/// - `ANIDB_CLIENT_VERSION`: Client version number
///
/// These can be set in a `.env` file in the working directory.
pub fn config_from_env() -> ApiConfig {
    let client_name = env::var(ENV_ANIDB_CLIENT).unwrap_or_default();
    let client_version = env::var(ENV_ANIDB_CLIENT_VERSION)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    ApiConfig::new(client_name, client_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize env var tests (they share global state)
    static ENV_TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_from_env_defaults() {
        let _lock = ENV_TEST_MUTEX.lock().unwrap();

        // Clear any existing env vars for this test
        env::remove_var(ENV_ANIDB_CLIENT);
        env::remove_var(ENV_ANIDB_CLIENT_VERSION);

        let config = config_from_env();

        assert!(config.client_name.is_empty());
        assert_eq!(config.client_version, 1);
        assert!(!config.is_configured());
    }

    #[test]
    fn test_config_from_env_with_values() {
        let _lock = ENV_TEST_MUTEX.lock().unwrap();

        env::set_var(ENV_ANIDB_CLIENT, "testclient");
        env::set_var(ENV_ANIDB_CLIENT_VERSION, "2");

        let config = config_from_env();

        assert_eq!(config.client_name, "testclient");
        assert_eq!(config.client_version, 2);
        assert!(config.is_configured());

        // Cleanup
        env::remove_var(ENV_ANIDB_CLIENT);
        env::remove_var(ENV_ANIDB_CLIENT_VERSION);
    }
}
