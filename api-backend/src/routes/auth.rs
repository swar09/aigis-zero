use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::auth, state::AppState};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(auth::login))
        .route("/me", get(auth::me))
}
