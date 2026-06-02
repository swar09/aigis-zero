use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;

use fleet_manager::{AgentRegistration, EnrollmentPort, RegistrationResult};

use crate::{
    store::{NodeRecord, NodeStore},
    token::sign_token,
};

/// Drives agent enrollment.
///
/// Holds an injected `NodeStore` and the JWT signing secret.
/// Stateless — safe to share across tasks via `Arc`.
pub struct NodeEnroller {
    store: Arc<dyn NodeStore>,
    jwt_secret: Vec<u8>,
}

impl NodeEnroller {
    /// Creates a new `NodeEnroller`.
    ///
    /// `jwt_secret` must match the secret in `grpc-listener`. A mismatch causes
    /// enrolled agents to be rejected on every subsequent RPC.
    #[must_use]
    pub fn new(store: Arc<dyn NodeStore>, jwt_secret: impl Into<Vec<u8>>) -> Self {
        Self {
            store,
            jwt_secret: jwt_secret.into(),
        }
    }
}

#[async_trait]
impl EnrollmentPort for NodeEnroller {
    /// Registers or re-registers an agent.
    ///
    /// 1. Delegates persistence to the injected `NodeStore` (upsert + audit log).
    /// 2. Signs a 24-hour JWT containing the assigned `node_id`.
    /// 3. Returns `RegistrationResult` to `grpc-listener`.
    ///
    /// Both store and signing failures map to `Status::internal`.
    /// Agents never receive internal error detail.
    async fn register_agent(&self, reg: AgentRegistration) -> Result<RegistrationResult, Status> {
        let node_id = self
            .store
            .upsert_node(NodeRecord {
                hostname: reg.hostname.clone(),
                os_version: reg.os_version,
                agent_version: reg.agent_version,
                machine_id: reg.machine_id.clone(),
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    err      = %e,
                    hostname = %reg.hostname,
                    "enrollment store failure"
                );
                Status::internal("enrollment failed")
            })?;

        let token = sign_token(&node_id, &self.jwt_secret).map_err(|e| {
            tracing::error!(err = %e, node_id = %node_id, "jwt signing failure");
            Status::internal("token signing failed")
        })?;

        tracing::info!(
            node_id    = %node_id,
            hostname   = %reg.hostname,
            machine_id = %reg.machine_id,
            "agent enrolled"
        );

        Ok(RegistrationResult { node_id, token })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{error::NodeEnrollmentError, store::NodeRecord};

    struct MockNodeStore {
        node_id: String,
    }

    #[async_trait]
    impl NodeStore for MockNodeStore {
        async fn upsert_node(&self, _: NodeRecord) -> Result<String, NodeEnrollmentError> {
            Ok(self.node_id.clone())
        }
    }

    struct FailingNodeStore;

    #[async_trait]
    impl NodeStore for FailingNodeStore {
        async fn upsert_node(&self, _: NodeRecord) -> Result<String, NodeEnrollmentError> {
            Err(NodeEnrollmentError::Store("simulated db failure".into()))
        }
    }

    fn reg() -> AgentRegistration {
        AgentRegistration {
            hostname: "test-host".into(),
            os_version: "Ubuntu 24.04".into(),
            agent_version: "0.1.0".into(),
            machine_id: "mid-test-001".into(),
        }
    }

    #[tokio::test]
    async fn successful_enrollment_returns_node_id_and_token() {
        let expected_id = "a1b2c3d4-0001-0000-0000-000000000001";
        let enroller = NodeEnroller::new(
            Arc::new(MockNodeStore {
                node_id: expected_id.into(),
            }),
            b"test-secret-long-enough".to_vec(),
        );

        let result = enroller
            .register_agent(reg())
            .await
            .expect("enrollment should succeed");
        assert_eq!(result.node_id, expected_id);
        assert!(!result.token.is_empty());
    }

    #[tokio::test]
    async fn store_failure_maps_to_internal_status() {
        let enroller = NodeEnroller::new(
            Arc::new(FailingNodeStore),
            b"test-secret-long-enough".to_vec(),
        );

        let err = enroller.register_agent(reg()).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::Internal);
    }
}
