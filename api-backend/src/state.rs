use std::sync::Arc;

use tokio::sync::broadcast;

use crate::{
    clients::FleetClient,
    config::Settings,
    db::{DbPool, create_pool},
    models::ws::LiveEvent,
    repositories::{DieselAlertRepository, DieselLogRepository, DieselNodeRepository},
    services::{AlertService, AuthService, LogService, NodeService},
};

pub struct AppStateInner {
    pub config: Arc<Settings>,
    pub auth_service: Arc<AuthService>,
    pub node_service: Arc<NodeService>,
    pub alert_service: Arc<AlertService>,
    pub log_service: Arc<LogService>,
    pub broadcast_tx: broadcast::Sender<LiveEvent>,
    pub nodes_pool: DbPool,
    pub alerts_pool: DbPool,
    pub logs_pool: DbPool,
}

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<AppStateInner>,
}

impl std::ops::Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AppState {
    pub fn new(config: Settings) -> anyhow::Result<Self> {
        let config = Arc::new(config);

        let nodes_pool = create_pool(&config.database_url_nodes, config.db_pool_max_size)?;
        let alerts_pool = create_pool(&config.database_url_alerts, config.db_pool_max_size)?;
        let logs_pool = create_pool(&config.database_url_logs, config.db_pool_max_size)?;

        let node_repo = Arc::new(DieselNodeRepository::new(nodes_pool.clone()));
        let alert_repo = Arc::new(DieselAlertRepository::new(alerts_pool.clone()));
        let log_repo = Arc::new(DieselLogRepository::new(logs_pool.clone()));

        let fleet_client = Arc::new(FleetClient::new(config.fleet_grpc_url.clone()));

        let auth_service = Arc::new(AuthService::new(config.clone()));
        let node_service = Arc::new(NodeService::new(node_repo, fleet_client));
        let alert_service = Arc::new(AlertService::new(alert_repo));
        let log_service = Arc::new(LogService::new(log_repo));

        let (broadcast_tx, _) = broadcast::channel(5000);

        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                auth_service,
                node_service,
                alert_service,
                log_service,
                broadcast_tx,
                nodes_pool,
                alerts_pool,
                logs_pool,
            }),
        })
    }
}
