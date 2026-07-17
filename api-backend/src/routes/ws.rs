use axum::{Router, routing::get};

use crate::{handlers::ws, state::AppState};

pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws::ws_handler))
}
