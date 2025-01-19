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
                // Handle specific database errors like constraint violations
                ScraperError::DatabaseError(format!("Database error: {}", db_err))
            }
            sqlx::Error::RowNotFound => {
                ScraperError::DatabaseError("Requested data not found".to_string())
            }
            sqlx::Error::ColumnNotFound(col_name) => {
                ScraperError::DatabaseError(format!("Column not found: {}", col_name))
            }
            sqlx::Error::ColumnDecode { index, source } => {
                ScraperError::DatabaseError(
                    format!("Failed to decode column {}: {}", index, source)
                )
            }
            sqlx::Error::Decode(desc) => {
                ScraperError::DatabaseError(format!("Decode error: {}", desc))
            }
            sqlx::Error::PoolTimedOut => {
                ScraperError::DatabaseError("Database connection pool timeout".to_string())
            }
            sqlx::Error::WorkerCrashed => {
                ScraperError::DatabaseError("Database worker thread crashed".to_string())
            }
            _ => ScraperError::DatabaseError(format!("Other database error: {}", err)),
        }
    }
}

impl From<MigrateError> for ScraperError {
    fn from(err: MigrateError) -> Self {
        match err {
            MigrateError::Source(source_err) => {
                // Handle errors that occurred during migration execution
                ScraperError::MigrationError(format!("Migration source error: {}", source_err))
            }
            MigrateError::VersionMismatch(version) => {
                // Handle version mismatch with the single version number provided
                ScraperError::MigrationError(format!("Migration version mismatch at version {}", version))
            }
            MigrateError::Dirty(version) => {
                // Handle cases where a migration failed and left the database in a "dirty" state
                ScraperError::MigrationError(
                    format!("Database left in dirty state at version {}", version)
                )
            }
            _ => ScraperError::MigrationError(format!("Other migration error: {}", err)),
        }
    }
}

pub type Result<T> = std::result::Result<T, ScraperError>;
