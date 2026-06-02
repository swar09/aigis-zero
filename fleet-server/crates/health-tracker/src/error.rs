use thiserror::Error;

/// All errors originating from the health-tracker crate.
#[derive(Debug, Error)]
pub enum HealthTrackerError {
    /// The underlying store rejected or failed the operation.
    #[error("store error: {0}")]
    Store(String),
}
