use thiserror::Error;

/// All errors that can originate from the `postgres-interface` crate.
#[derive(Debug, Error)]
pub enum PgError {
    /// A sqlx query or pool operation failed.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// sqlx migration failed at startup.
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}
