//! Use case `GET /trace/:id`: трассировка гипотезы до claims и ячеек xlsx.

use contracts::TraceReport;

use crate::application::error::UseCaseError;
use crate::application::ports::RunRepository;
use crate::domain::trace;

pub fn execute(
    runs: &dyn RunRepository,
    run_id: Option<String>,
    hypothesis_id: &str,
) -> Result<TraceReport, UseCaseError> {
    let run = super::pick_run(runs, run_id)?;
    let hyp = run
        .board
        .hypotheses
        .iter()
        .find(|h| h.id == hypothesis_id)
        .ok_or_else(|| UseCaseError::NotFound(format!("hypothesis '{hypothesis_id}' not found")))?;
    Ok(trace::trace(hyp, &run.extract, &run.diagnostics))
}
