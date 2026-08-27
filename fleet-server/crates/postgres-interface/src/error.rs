use thiserror::Error;

/// All errors that can originate from the `postgres-interface` crate.
#[derive(Debug, Error)]
pub enum PgError {
    #[error("database query error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("connection pool error: {0}")]
    Pool(#[from] diesel_async::pooled_connection::deadpool::PoolError),

    #[error("pool configuration error: {0}")]
    PoolConfig(String),
}
