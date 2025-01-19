use std::fmt;
use std::error::Error;
use sqlx::migrate::MigrateError;

#[derive(Debug)]
pub enum ScraperError {
    ApiError(String),
    ParseError(String),
    ValidationError(String),
    RateLimitError(String),
    NetworkError(String),
    IoError(String),
    DatabaseError(String),
    MigrationError(String),
}

impl fmt::Display for ScraperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScraperError::ApiError(msg) => write!(f, "API Error: {}", msg),
            ScraperError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
            ScraperError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            ScraperError::RateLimitError(msg) => write!(f, "Rate Limit Error: {}", msg),
            ScraperError::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            ScraperError::IoError(msg) => write!(f, "IO Error: {}", msg),
            ScraperError::DatabaseError(msg) => write!(f, "Database Error: {}", msg),
            ScraperError::MigrationError(msg) => write!(f, "Migration Error: {}", msg),
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

impl From<std::io::Error> for ScraperError {
    fn from(err: std::io::Error) -> Self {
        ScraperError::IoError(err.to_string())
    }
}

impl From<sqlx::Error> for ScraperError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db_err) => {
                ScraperError::DatabaseError(format!("Database error: {}", db_err))
            }
            sqlx::Error::RowNotFound => {
                ScraperError::DatabaseError("Requested data not found".to_string())
            }
            sqlx::Error::Protocol(msg) => {
                ScraperError::DatabaseError(format!("Database protocol error: {}", msg))
            }
            sqlx::Error::Io(io_err) => {
                ScraperError::IoError(io_err.to_string())
            }
            _ => ScraperError::DatabaseError(err.to_string()),
        }
    }
}

impl From<MigrateError> for ScraperError {
    fn from(err: MigrateError) -> Self {
        match err {
            MigrateError::Source(source_err) => {
                ScraperError::MigrationError(format!("Migration source error: {}", source_err))
            }
            MigrateError::VersionMismatch => {
                ScraperError::MigrationError("Migration version mismatch".to_string())
            }
            _ => ScraperError::MigrationError(format!("Migration failed: {}", err)),
        }
    }
}

pub type Result<T> = std::result::Result<T, ScraperError>;