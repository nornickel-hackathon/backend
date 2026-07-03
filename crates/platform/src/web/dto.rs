//! HTTP-DTO входа. Тела ответов — типы `contracts`, повторно не объявляются.

use contracts::KpiContract;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RunRequest {
    pub kpi_contract: KpiContract,
    #[serde(default)]
    pub pack_id: Option<String>,
}

#[derive(Deserialize)]
pub struct BoardQuery {
    // Часть контракта `GET /board?run_id=...`; принимается, но пока не влияет на
    // выборку (отдаётся последний прогон) — поведение сохранено как было.
    #[serde(default)]
    #[allow(dead_code)]
    pub run_id: Option<String>,
}
