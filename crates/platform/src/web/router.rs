//! Сборка axum-роутера платформы (HTTP-шов web ↔ platform).

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
        .route("/constraints/parse", post(handlers::parse_constraints))
        .route("/extract", get(handlers::extract))
        .route("/expert_hypotheses", get(handlers::expert_hypotheses))
        .route("/benchmark", get(handlers::benchmark))
        .route("/data_readiness", get(handlers::data_readiness))
        .route("/trace/:id", get(handlers::trace))
        .route("/roadmap", get(handlers::roadmap))
        .route("/factories", get(handlers::factories))
        .route("/export/board.json", get(handlers::export_board_json))
        .route("/export/board.csv", get(handlers::export_board_csv))
        .layer(from_fn(middleware::contract_version_header))
        .with_state(state)
}
