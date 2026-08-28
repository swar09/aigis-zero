use std::time::Duration;

use deadpool_diesel::{Runtime, Timeouts};
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{
        AsyncDieselConnectionManager,
        deadpool::{Object, Pool},
    },
};

use crate::error::PgError;

/// Type alias for the async diesel deadpool `PostgreSQL` connection pool.
///
/// Thread-safe and cheaply cloneable (`Arc` inside).
pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn = Object<AsyncPgConnection>;

/// Creates an async diesel connection pool for `PostgreSQL`.
///
/// Bounded pool size with fast failover timeouts prevents connection and thread exhaustion.
///
/// # Errors
///
/// Returns `PgError::PoolConfig` if pool initialization fails.
pub fn create_pool(database_url: &str, max_size: usize) -> Result<DbPool, PgError> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let timeouts = Timeouts {
        wait: Some(Duration::from_secs(3)),
        create: Some(Duration::from_secs(5)),
        recycle: Some(Duration::from_secs(2)),
    };

    let pool = Pool::builder(config)
        .max_size(max_size)
        .timeouts(timeouts)
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|e| PgError::PoolConfig(e.to_string()))?;

    tracing::info!(
        database_url,
        max_size,
        "Diesel async PostgreSQL connection pool initialized"
    );
    Ok(pool)
}

/// Helper connecting with default pool size (5).
///
/// # Errors
///
/// Returns `PgError` if pool creation fails.
pub async fn connect(database_url: &str) -> Result<DbPool, PgError> {
    create_pool(database_url, 5)
}
