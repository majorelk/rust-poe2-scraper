use thiserror::Error;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
#[allow(dead_code)]
pub enum ScraperError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Rate limit error: {0}")]
    RateLimitError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),
}

impl From<serde_json::Error> for ScraperError {
    fn from(err: serde_json::Error) -> Self {
        ScraperError::ParseError(err.to_string())
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
            sqlx::Error::ColumnNotFound(col_name) => {
                ScraperError::DatabaseError(format!("Column not found: {}", col_name))
            }
            sqlx::Error::ColumnDecode { index, source } => ScraperError::DatabaseError(format!(
                "Failed to decode column {}: {}",
                index, source
            )),
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

impl From<sqlx::migrate::MigrateError> for ScraperError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        match err {
            sqlx::migrate::MigrateError::Source(source_err) => {
                ScraperError::MigrationError(format!("Migration source error: {}", source_err))
            }
            sqlx::migrate::MigrateError::VersionMismatch(version) => ScraperError::MigrationError(
                format!("Migration version mismatch at version {}", version),
            ),
            sqlx::migrate::MigrateError::Dirty(version) => ScraperError::MigrationError(format!(
                "Database left in dirty state at version {}",
                version
            )),
            _ => ScraperError::MigrationError(format!("Other migration error: {}", err)),
        }
    }
}

pub type Result<T> = std::result::Result<T, ScraperError>;
