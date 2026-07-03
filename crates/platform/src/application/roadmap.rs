//! Use case `GET /roadmap`: рекомендованный план действий по прогону с честной
//! де-дубликацией стоимости и бюджетом `max_capex` (1..3).

use contracts::RoadmapPlan;

use crate::application::error::UseCaseError;
use crate::application::ports::RunRepository;
use crate::domain::roadmap;

pub fn execute(
    runs: &dyn RunRepository,
    run_id: Option<String>,
    max_capex: u8,
) -> Result<RoadmapPlan, UseCaseError> {
    let run = super::pick_run(runs, run_id)?;
    let max_capex = max_capex.clamp(1, 3);
    Ok(roadmap::build(
        &run.board.hypotheses,
        &run.extract.entities,
        &run.contract.factory_id,
        max_capex,
    ))
}
