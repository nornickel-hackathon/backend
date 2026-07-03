//! Use case `GET /data_readiness`: качество исходных данных прогона.

use contracts::DataReadiness;

use crate::application::error::UseCaseError;
use crate::application::ports::RunRepository;
use crate::domain::readiness;

pub fn execute(
    runs: &dyn RunRepository,
    run_id: Option<String>,
) -> Result<DataReadiness, UseCaseError> {
    let run = super::pick_run(runs, run_id)?;
    Ok(readiness::readiness(&run.diagnostics))
}
