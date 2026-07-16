pub mod alerts;
pub mod auth;
pub mod health;
pub mod logs;
pub mod nodes;
pub mod ws;

use axum::Router;

use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let api_v1 = Router::new()
        .nest("/auth", auth::auth_routes())
        .nest("/nodes", nodes::node_routes())
        .nest("/alerts", alerts::alert_routes())
        .nest("/logs", logs::log_routes())
        .merge(ws::ws_routes());

    Router::new()
        .merge(health::health_routes())
        .nest("/api/v1", api_v1)
        .with_state(state)
}
