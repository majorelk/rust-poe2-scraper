use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum ScraperError {
    ApiError(String),
    ParseError(String),
    ValidationError(String),
    RateLimitError(String),
    NetworkError(String),
}

impl fmt::Display for ScraperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScraperError::ApiError(msg) => write!(f, "API Error: {}", msg),
            ScraperError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
            ScraperError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            ScraperError::RateLimitError(msg) => write!(f, "Rate Limit Error: {}", msg),
            ScraperError::NetworkError(msg) => write!(f, "Network Error: {}", msg),
        }
    }
}

impl Error for ScraperError {}

impl From<reqwest::Error> for ScraperError {
    fn from(err: reqwest::Error) -> Self {
        ScraperError::NetworkError(err.to_string())
    }
}

impl From<serde_json::Error> for ScraperError {
    fn from(err: serde_json::Error) -> Self {
        ScraperError::ParseError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ScraperError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = ScraperError::ApiError("test error".to_string());
        assert_eq!(error.to_string(), "API Error: test error");
    }
}