#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod health_store;
pub mod models;
pub mod node_store;
pub mod pool;
pub mod schema;

pub use error::PgError;
pub use health_store::PgHealthStore;
pub use node_store::PgNodeStore;
pub use pool::{DbPool, connect, create_pool};
