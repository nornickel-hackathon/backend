//! Domain layer — бизнес-правила платформы. Чистые функции над типами
//! `contracts`: без I/O, без HTTP, без axum. Внутренний круг (Clean
//! Architecture) — ни от чего в этом крейте не зависит.

pub mod annotate;
pub mod benchmark;
pub mod money;
pub mod readiness;
pub mod rerun;
pub mod roadmap;
pub mod snapshot;
pub mod trace;
pub mod validation;
