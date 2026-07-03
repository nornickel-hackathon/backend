//! Application layer — use cases + порты. Оркестрирует `domain` и `engine`
//! через абстрактные порты (dependency inversion): use case знает трейт, а не
//! файл/БД. Не зависит от axum/HTTP и от конкретной инфраструктуры.

pub mod benchmark;
pub mod board;
pub mod error;
pub mod export;
pub mod factories;
pub mod hypothesis;
pub mod ports;
pub mod readiness;
pub mod rerun;
pub mod roadmap;
pub mod run;
pub mod run_record;
pub mod trace;

pub use error::UseCaseError;

use crate::application::ports::RunRepository;
use crate::application::run_record::RunRecord;

/// Выбрать прогон по `run_id` (или последний). Общий помощник read-only use case'ов.
pub(crate) fn pick_run(
    runs: &dyn RunRepository,
    run_id: Option<String>,
) -> Result<RunRecord, UseCaseError> {
    match run_id {
        Some(id) => runs
            .get(&id)
            .ok_or_else(|| UseCaseError::NotFound(format!("run '{id}' not found"))),
        None => runs
            .last()
            .ok_or_else(|| UseCaseError::NotFound("no run yet; call POST /run first".to_string())),
    }
}
