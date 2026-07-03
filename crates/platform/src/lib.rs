//! crates/platform — Rust-платформа (Роль 1) на чистой архитектуре.
//!
//! Зависимости направлены внутрь (Clean Architecture):
//! - [`domain`]         — бизнес-правила платформы (validate/snapshot/rerun), чистые.
//! - [`application`]    — use cases + порты (dependency inversion).
//! - [`infrastructure`] — файловые/in-memory адаптеры портов.
//! - [`web`]            — axum-хендлеры (HTTP delivery).
//! - [`state`]          — composition root (DI-контейнер).
//!
//! Граница Python↔Rust — только JSON (без PyO3). Зовёт engine как библиотеку.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod state;
pub mod web;

/// Обратная совместимость публичного пути `platform::validate::validate`.
pub mod validate {
    pub use crate::domain::validation::validate;
}

pub const CONTRACT_VERSION: &str = "1";

pub use web::build_router;
