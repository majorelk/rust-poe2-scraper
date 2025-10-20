use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_trade_base_url")]
    pub trade_base_url: String,

    #[serde(default = "default_user_agent")]
    #[allow(dead_code)]
    pub user_agent: String,

    #[serde(default = "default_requests_per_min")]
    pub requests_per_min: u32,

    #[serde(default = "default_db_url")]
    #[allow(dead_code)]
    pub db_url: String,

    #[serde(default = "default_rate_limit_delay_ms")]
    pub rate_limit_delay_ms: u64,

    #[serde(default = "default_log_format")]
    pub log_format: String,
}

fn default_trade_base_url() -> String {
    "https://www.pathofexile.com".to_string()
}

fn default_user_agent() -> String {
    "POE2-Scraper/0.1.0 (https://github.com/majorelk/rust-poe2-scraper)".to_string()
}

fn default_requests_per_min() -> u32 {
    60
}

fn default_db_url() -> String {
    "sqlite:poe_items.db".to_string()
}

fn default_rate_limit_delay_ms() -> u64 {
    500
}

fn default_log_format() -> String {
    "pretty".to_string()
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenvy::dotenv().ok(); // Load .env file if it exists, ignore if not
        
        let config = envy::prefixed("").from_env::<Config>()?;
        Ok(config)
    }

    /// Get configuration with defaults for any missing values
    pub fn load() -> Self {
        Self::from_env().unwrap_or_else(|_| Self::default())
    }

    /// Calculate rate limit delay as Duration
    #[allow(dead_code)]
    pub fn rate_limit_delay(&self) -> Duration {
        Duration::from_millis(self.rate_limit_delay_ms)
    }

    /// Calculate delay between requests from requests_per_min
    #[allow(dead_code)]
    pub fn delay_from_rpm(&self) -> Duration {
        let delay_ms = 60_000 / self.requests_per_min as u64;
        Duration::from_millis(delay_ms)
    }

    /// Log a sanitized summary of the config (no secrets)
    pub fn log_summary(&self) {
        tracing::info!(
            trade_base_url = %self.trade_base_url,
            requests_per_min = self.requests_per_min,
            rate_limit_delay_ms = self.rate_limit_delay_ms,
            log_format = %self.log_format,
            "Configuration loaded"
        );
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trade_base_url: default_trade_base_url(),
            user_agent: default_user_agent(),
            requests_per_min: default_requests_per_min(),
            db_url: default_db_url(),
            rate_limit_delay_ms: default_rate_limit_delay_ms(),
            log_format: default_log_format(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.trade_base_url, "https://www.pathofexile.com");
        assert_eq!(config.requests_per_min, 60);
        assert_eq!(config.db_url, "sqlite:poe_items.db");
        assert_eq!(config.rate_limit_delay_ms, 500);
    }

    #[test]
    fn test_rate_limit_delay() {
        let config = Config::default();
        assert_eq!(config.rate_limit_delay(), Duration::from_millis(500));
    }

    #[test]
    fn test_delay_from_rpm() {
        let mut config = Config::default();
        config.requests_per_min = 60;
        assert_eq!(config.delay_from_rpm(), Duration::from_millis(1000));

        config.requests_per_min = 120;
        assert_eq!(config.delay_from_rpm(), Duration::from_millis(500));
    }

    #[test]
    fn test_missing_vars_fall_back_to_defaults() {
        // This test ensures that if environment variables are missing,
        // we fall back to sensible defaults
        let config = Config::load();
        assert!(!config.trade_base_url.is_empty());
        assert!(!config.user_agent.is_empty());
        assert!(config.requests_per_min > 0);
        assert!(!config.db_url.is_empty());
    }
}
