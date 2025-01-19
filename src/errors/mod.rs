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
        // Convert different types of SQLx errors to appropriate ScraperError variants
        match err {
            sqlx::Error::Database(db_err) => {
                // Handle database-specific errors (like constraint violations)
                ScraperError::DatabaseError(format!("Database error: {}", db_err))
            }
            sqlx::Error::RowNotFound => {
                ScraperError::DatabaseError("Requested data not found".to_string())
            }
            sqlx::Error::Protocol(msg) => {
                ScraperError::DatabaseError(format!("Database protocol error: {}", msg))
            }
            sqlx::Error::Io(io_err) => {
                // Io errors during database operations
                ScraperError::IoError(io_err.to_string())
            }
            // Catch all other database errors
            _ => ScraperError::DatabaseError(err.to_string()),
        }
    }
}

impl From<MigrateError> for ScraperError {
    fn from(err: MigrateError) -> Self {
        match err {
            MigrateError::Source(source_err) => {
                // Source errors are usually database errors that occurred during migration
                ScraperError::MigrationError(format!("Migration source error: {}", source_err))
            }
            MigrateError::ChecksumMismatch { version, .. } => {
                // This happens when a migration file has been modified after being applied
                ScraperError::MigrationError(
                    format!("Migration checksum mismatch for version {}", version)
                )
            }
            MigrateError::VersionMismatch(applied, latest) => {
                // This occurs when there's a version number conflict
                ScraperError::MigrationError(
                    format!("Migration version mismatch: applied={}, latest={}", applied, latest)
                )
            }
            // Handle all other migration errors with their specific messages
            _ => ScraperError::MigrationError(format!("Migration failed: {}", err)),
        }
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