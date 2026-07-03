//! HTTP-DTO входа/обёртки ответа. Тела портфеля/гипотез — типы `contracts`.

use contracts::{BoardResponse, KpiContract, RerunAction};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RunRequest {
    pub factory_id: String,
    #[serde(default)]
    pub pack_id: Option<String>,
    /// Optional xlsx path for a newly provided/hidden factory. When set, platform
    /// must ask the live sidecar for diagnostics instead of requiring a fixture.
    #[serde(default)]
    pub source_file: Option<String>,
    /// Опционален — дефолтный контракт по factory_id, если не задан.
    #[serde(default)]
    pub kpi_contract: Option<KpiContract>,
}

/// Обёртка ответа `POST /run` — `{ run_id, board }` (HTTP-шов web ↔ platform).
#[derive(Serialize)]
pub struct RunResponse {
    pub run_id: String,
    pub board: BoardResponse,
}

#[derive(Deserialize)]
pub struct BoardQuery {
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Deserialize)]
pub struct RoadmapQuery {
    #[serde(default)]
    pub run_id: Option<String>,
    /// Бюджет: максимальный capex_class действий (1..3). По умолчанию 3.
    #[serde(default)]
    pub max_capex: Option<u8>,
}

#[derive(Deserialize)]
pub struct RerunRequest {
    #[serde(default)]
    pub run_id: Option<String>,
    pub action: RerunAction,
}
