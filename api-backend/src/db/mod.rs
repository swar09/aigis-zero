pub mod schema;

use std::time::Duration;

use deadpool_diesel::Timeouts;
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{
        AsyncDieselConnectionManager,
        deadpool::{Object, Pool},
    },
};

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn = Object<AsyncPgConnection>;

pub fn create_pool(database_url: &str, max_size: usize) -> anyhow::Result<DbPool> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let timeouts = Timeouts {
        wait: Some(Duration::from_secs(3)),
        create: Some(Duration::from_secs(5)),
        recycle: Some(Duration::from_secs(2)),
    };

    let pool = Pool::builder(config)
        .max_size(max_size)
        .timeouts(timeouts)
        .build()?;
    Ok(pool)
}
