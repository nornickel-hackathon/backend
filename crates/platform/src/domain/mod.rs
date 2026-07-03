//! Domain layer — бизнес-правила платформы. Чистые функции над типами
//! `contracts`: без I/O, без HTTP, без axum. Внутренний круг (Clean
//! Architecture) — ни от чего в этом крейте не зависит.

pub mod rerun;
pub mod snapshot;
pub mod validation;
