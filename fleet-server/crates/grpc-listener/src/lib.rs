#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod auth;
pub mod config;
pub mod error;
pub mod server;
pub mod service;

pub use config::GrpcListenerConfig;
pub use error::Error;
pub use server::{GrpcServer, shutdown_signal};
pub use service::FleetServiceImpl;
