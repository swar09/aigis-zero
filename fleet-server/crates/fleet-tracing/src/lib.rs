#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod init;

pub use config::{LogFormat, TracingConfig};
pub use init::{InitError, init};
