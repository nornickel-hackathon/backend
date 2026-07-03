//! Порты application-слоя (dependency inversion). Use cases зависят от этих
//! трейтов; конкретные реализации живут в `infrastructure`.

use contracts::{BoardResponse, DomainPack, ExtractResponse};

use crate::application::run_record::RunRecord;

/// Источник extract-результата сайдкара (в MVP — файловая фикстура).
pub trait ExtractSource: Send + Sync {
    fn load(&self) -> Result<ExtractResponse, String>;
}

/// Хранилище доменных паков (`packs/<pack_id>.yaml`).
pub trait PackRepository: Send + Sync {
    fn load(&self, pack_id: &str) -> Result<DomainPack, String>;
}

/// Стор прогонов: последний прогон нужен для /board, /hypothesis, /rerun.
pub trait RunRepository: Send + Sync {
    fn store(&self, run: RunRecord);
    fn last(&self) -> Option<RunRecord>;
}

/// Fallback-портфель до первого /run (в MVP — `fixtures/board.json`).
pub trait BoardGateway: Send + Sync {
    fn load(&self) -> Result<BoardResponse, String>;
}
