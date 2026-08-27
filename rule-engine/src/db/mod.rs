pub mod schema;

use std::time::Duration;

use deadpool_diesel::{Runtime, Timeouts};
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{
        AsyncDieselConnectionManager,
        deadpool::{Object, Pool},
    },
};

use crate::error::AppError;

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn = Object<AsyncPgConnection>;

/// Builds a high-throughput connection pool with configurable maximum connections and fail-fast timeouts
pub fn create_pool(database_url: &str, max_connections: usize) -> Result<DbPool, AppError> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

    let timeouts = Timeouts {
        wait: Some(Duration::from_secs(3)),
        create: Some(Duration::from_secs(5)),
        recycle: Some(Duration::from_secs(2)),
    };

    Pool::builder(config)
        .max_size(max_connections)
        .timeouts(timeouts)
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|e| AppError::DatabasePool(format!("Failed to build connection pool: {e}")))
}
