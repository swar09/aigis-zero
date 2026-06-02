#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod store;
pub mod tracker;

pub use error::HealthTrackerError;
pub use store::{HealthStore, HeartbeatRecord};
pub use tracker::HealthTracker;
