//! Application layer — use cases + порты. Оркестрирует `domain` и `engine`
//! через абстрактные порты (dependency inversion): use case знает трейт, а не
//! файл/БД. Не зависит от axum/HTTP и от конкретной инфраструктуры.

pub mod board;
pub mod error;
pub mod hypothesis;
pub mod ports;
pub mod rerun;
pub mod run;
pub mod run_record;

pub use error::UseCaseError;
