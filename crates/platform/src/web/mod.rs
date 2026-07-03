//! Web — interface adapters (HTTP delivery через axum). Тонкие хендлеры
//! транслируют HTTP ↔ use cases; бизнес-логики здесь нет.

mod dto;
mod error;
mod handlers;
mod middleware;
mod router;

pub use router::build_router;
