use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::error::PgError;

/// Creates a connection pool and runs all pending sqlx migrations.
///
/// Call exactly once at process startup. The returned pool is cheaply
/// cloneable (`Arc` inside) — pass it by value to `PgNodeStore` and
/// `PgHealthStore`.
///
/// # Errors
///
/// Returns `PgError::Database` if the connection cannot be established
/// within the 5-second `acquire_timeout`.
/// Returns `PgError::Migration` if any migration SQL fails.
pub async fn connect(database_url: &str) -> Result<PgPool, PgError> {
    let pool = PgPoolOptions::new()
        // Sane default for a single fleet-server instance.
        // Expose this as a config key once you have measured concurrency.
        .max_connections(5)
        // Hard fail at startup rather than queue requests silently.
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;

    // Migrations are embedded in the binary at compile time via this macro.
    // The path is relative to this crate's Cargo.toml:
    //   fleet-server/crates/postgres-interface/ → ../../migrations
    //   = fleet-server/migrations/
    // At runtime there is no file dependency — the SQL is in the binary.
    sqlx::migrate!("../../migrations").run(&pool).await?;

    tracing::info!("postgres pool connected and migrations applied");
    Ok(pool)
}
