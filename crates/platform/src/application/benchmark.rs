//! Use case `GET /benchmark`: сверка портфеля прогона с эталонными гипотезами
//! экспертов. Плюс общий парсер golden-набора для run/rerun.

use contracts::{BenchmarkReport, ExpertHypothesis};

use crate::application::error::UseCaseError;
use crate::application::ports::{ExpertHypothesesGateway, RunRepository};
use crate::domain::benchmark;

/// Типизированный golden-набор из `ExpertHypothesesGateway` (мягко: ошибка -> пусто).
pub fn load_experts(gw: &dyn ExpertHypothesesGateway) -> Vec<ExpertHypothesis> {
    match gw.load() {
        Ok(v) => serde_json::from_value(v).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn execute(
    runs: &dyn RunRepository,
    experts_gw: &dyn ExpertHypothesesGateway,
    run_id: Option<String>,
) -> Result<BenchmarkReport, UseCaseError> {
    let run = super::pick_run(runs, run_id)?;
    let experts = load_experts(experts_gw);
    let mut hyps = run.board.hypotheses.clone();
    let report = benchmark::match_experts(
        &mut hyps,
        &run.extract.entities,
        &experts,
        &run.contract.factory_id,
    );
    Ok(report)
}
