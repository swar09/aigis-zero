use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::nodes, state::AppState};

pub fn node_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(nodes::list_nodes))
        .route("/{id}", get(nodes::get_node_by_id))
        .route("/{id}/isolate", post(nodes::isolate_node))
        .route("/{id}/unisolate", post(nodes::unisolate_node))
}
