//! crates/platform — Rust-платформа (Роль 1). axum-API поверх engine + fixtures.
//! Граница Python↔Rust — только JSON (без PyO3). Зовёт engine как библиотеку.

pub mod pack;
pub mod rerun;
pub mod routes;
pub mod snapshot;
pub mod state;
pub mod validate;

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;

use state::AppState;

pub const CONTRACT_VERSION: &str = "1";

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/run", post(routes::run))
        .route("/board", get(routes::board))
        .route("/hypothesis/:id", get(routes::hypothesis))
        .route("/rerun", post(routes::rerun))
        .layer(middleware::from_fn(contract_version_header))
        .with_state(state)
}

/// Проставляет заголовок версии контракта на все ответы (API_CONVENTIONS.md).
async fn contract_version_header(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut()
        .insert("X-Contract-Version", HeaderValue::from_static(CONTRACT_VERSION));
    res
}
