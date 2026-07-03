//! Порты application-слоя (dependency inversion). Use cases зависят от этих
//! трейтов; конкретные реализации живут в `infrastructure`.

use contracts::{BoardResponse, DiagnosticsReport, DomainPack, ExtractResponse, FactoryConfig};
use serde_json::Value;

use crate::application::run_record::RunRecord;

/// Источник extract-результата сайдкара (в MVP — файловая фикстура).
pub trait ExtractSource: Send + Sync {
    fn load(&self) -> Result<ExtractResponse, String>;
}

/// Источник диагностики хвостов по фабрике. Для известных фабрик это может быть
/// `fixtures/diagnostics_<factory_id>.json`; для hidden factory `source_file`
/// позволяет обратиться к живому sidecar без обязательной фикстуры.
pub trait DiagnosticsSource: Send + Sync {
    fn load(
        &self,
        factory_id: &str,
        source_file: Option<&str>,
        pack_id: &str,
    ) -> Result<DiagnosticsReport, String>;
}

/// Репозиторий конфигов фабрик (`factories/<factory_id>.yaml`).
pub trait FactoryRepository: Send + Sync {
    fn load(&self, factory_id: &str) -> Result<FactoryConfig, String>;
}

/// Хранилище доменных паков (`packs/<pack_id>.yaml`).
pub trait PackRepository: Send + Sync {
    fn load(&self, pack_id: &str) -> Result<DomainPack, String>;
}

/// Стор прогонов: нужен для /board, /hypothesis, /rerun (по run_id и последний).
pub trait RunRepository: Send + Sync {
    /// Выделить следующий стабильный `run_id` (например `run_0001`).
    fn next_run_id(&self) -> String;
    fn store(&self, run: RunRecord);
    fn get(&self, run_id: &str) -> Option<RunRecord>;
    fn last(&self) -> Option<RunRecord>;
}

/// Fallback-портфель до первого /run (в MVP — `fixtures/board.json`).
pub trait BoardGateway: Send + Sync {
    fn load(&self) -> Result<BoardResponse, String>;
}

/// Отдаёт `golden/expert_hypotheses.json` как есть (фронту для Benchmark view).
pub trait ExpertHypothesesGateway: Send + Sync {
    fn load(&self) -> Result<Value, String>;
}
