use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tonic::Status;

use fleet_manager::{AgentHeartbeat, HeartbeatPort};

use crate::store::{HealthStore, HeartbeatRecord};

/// Records agent heartbeats.
///
/// Stateless beyond the injected store. Safe to share via `Arc`.
pub struct HealthTracker {
    store: Arc<dyn HealthStore>,
}

impl HealthTracker {
    #[must_use]
    pub fn new(store: Arc<dyn HealthStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl HeartbeatPort for HealthTracker {
    /// Stamps the heartbeat with server time and delegates to the store.
    ///
    /// Maps `agent_status` from the agent's reported status field.
    /// The agent cannot report `"isolated"` — only `"healthy"` or `"degraded"`
    /// are accepted. Any other value is coerced to `"degraded"` to be safe.
    async fn record_heartbeat(&self, hb: AgentHeartbeat) -> Result<(), Status> {
        // Sanitize: only accept known agent-reportable statuses.
        // 'isolated' is an OPERATOR concept — agents cannot self-report it.
        let agent_status = match hb.status.as_str() {
            "healthy" => "healthy".to_string(),
            "degraded" => "degraded".to_string(),
            other => {
                tracing::warn!(
                    node_id = %hb.node_id,
                    reported = %other,
                    "unknown agent status — coercing to degraded"
                );
                "degraded".to_string()
            }
        };

        tracing::debug!(
            node_id         = %hb.node_id,
            agent_status    = %agent_status,
            events_buffered = hb.events_buffered,
            "heartbeat received"
        );

        self.store
            .record_heartbeat(HeartbeatRecord {
                node_id: hb.node_id.clone(),
                agent_status,
                events_buffered: hb.events_buffered,
                recorded_at: Utc::now(),
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    err     = %e,
                    node_id = %hb.node_id,
                    "heartbeat store failure"
                );
                Status::internal("heartbeat store failed")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{error::HealthTrackerError, store::HeartbeatRecord};
    use std::sync::Mutex;

    struct MockHealthStore {
        calls: Mutex<Vec<HeartbeatRecord>>,
    }

    impl MockHealthStore {
        fn new() -> Self {
            Self {
                calls: Mutex::new(vec![]),
            }
        }

        fn call_count(&self) -> usize {
            // Lock poisoning only happens if a test panicked while holding it.
            // Recovering the inner value is correct here.
            self.calls.lock().unwrap_or_else(|p| p.into_inner()).len()
        }

        fn last_call(&self) -> Option<HeartbeatRecord> {
            self.calls
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .last()
                .cloned()
        }
    }

    #[async_trait]
    impl HealthStore for MockHealthStore {
        async fn record_heartbeat(
            &self,
            record: HeartbeatRecord,
        ) -> Result<(), HealthTrackerError> {
            self.calls
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(record);
            Ok(())
        }
    }

    struct FailingHealthStore;

    #[async_trait]
    impl HealthStore for FailingHealthStore {
        async fn record_heartbeat(&self, _: HeartbeatRecord) -> Result<(), HealthTrackerError> {
            Err(HealthTrackerError::Store("simulated failure".into()))
        }
    }

    fn hb(status: &str) -> AgentHeartbeat {
        AgentHeartbeat {
            node_id: "a1b2c3d4-0001-0000-0000-000000000001".into(),
            status: status.into(),
            events_buffered: 42,
        }
    }

    #[tokio::test]
    async fn heartbeat_forwarded_to_store() {
        let store = Arc::new(MockHealthStore::new());
        let tracker = HealthTracker::new(Arc::clone(&store) as Arc<dyn HealthStore>);

        tracker
            .record_heartbeat(hb("healthy"))
            .await
            .expect("should succeed");

        assert_eq!(store.call_count(), 1);
    }

    #[tokio::test]
    async fn healthy_status_passes_through_unchanged() {
        let store = Arc::new(MockHealthStore::new());
        let tracker = HealthTracker::new(Arc::clone(&store) as Arc<dyn HealthStore>);

        tracker
            .record_heartbeat(hb("healthy"))
            .await
            .expect("should succeed");

        let call = store.last_call().expect("should have one call");
        assert_eq!(call.agent_status, "healthy");
    }

    #[tokio::test]
    async fn isolated_status_from_agent_is_coerced_to_degraded() {
        // Agents must never be able to self-report 'isolated' — that is operator-only.
        let store = Arc::new(MockHealthStore::new());
        let tracker = HealthTracker::new(Arc::clone(&store) as Arc<dyn HealthStore>);

        tracker
            .record_heartbeat(hb("isolated"))
            .await
            .expect("should succeed");

        let call = store.last_call().expect("should have one call");
        assert_eq!(
            call.agent_status, "degraded",
            "'isolated' from agent must be coerced to 'degraded'"
        );
    }

    #[tokio::test]
    async fn record_contains_server_timestamp() {
        let before = Utc::now();
        let store = Arc::new(MockHealthStore::new());
        let tracker = HealthTracker::new(Arc::clone(&store) as Arc<dyn HealthStore>);

        tracker
            .record_heartbeat(hb("healthy"))
            .await
            .expect("should succeed");

        let after = Utc::now();
        let call = store.last_call().expect("should have one call");
        assert!(call.recorded_at >= before && call.recorded_at <= after);
    }

    #[tokio::test]
    async fn store_failure_maps_to_internal_status() {
        let tracker = HealthTracker::new(Arc::new(FailingHealthStore));

        let err = tracker.record_heartbeat(hb("healthy")).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::Internal);
    }
}
