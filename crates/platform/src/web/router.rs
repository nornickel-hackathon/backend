//! Сборка axum-роутера платформы.

use axum::middleware::from_fn;
use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;
use crate::web::{handlers, middleware};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/run", post(handlers::run))
        .route("/board", get(handlers::board))
        .route("/hypothesis/:id", get(handlers::hypothesis))
        .route("/rerun", post(handlers::rerun))
        .layer(from_fn(middleware::contract_version_header))
        .with_state(state)
}
