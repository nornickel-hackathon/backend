//! Use case `POST /rerun`: применить action к контракту конкретного прогона и
//! пересчитать БЕЗ extraction. Тот же snapshot/граф — воспроизводимость входа
//! сохраняется; `change_price` пересчитывает деньги и ranking (engine).

use contracts::{ApiError, BoardResponse, RerunAction};
use serde_json::Value;

use crate::application::benchmark::load_experts;
use crate::application::error::UseCaseError;
use crate::application::ports::{ExpertHypothesesGateway, RunRepository};
use crate::domain::{benchmark, rerun};

pub fn execute(
    runs: &dyn RunRepository,
    experts_gw: &dyn ExpertHypothesesGateway,
    run_id: Option<String>,
    action: RerunAction,
) -> Result<BoardResponse, UseCaseError> {
    let mut run = match &run_id {
        Some(id) => runs
            .get(id)
            .ok_or_else(|| UseCaseError::NotFound(format!("run '{id}' not found")))?,
        None => runs
            .last()
            .ok_or_else(|| UseCaseError::NotFound("no run yet; call POST /run first".to_string()))?,
    };

    rerun::apply(&mut run.contract, &action);

    // Граф переиспользуем из уже проаннотированного extract прогона.
    let graph = engine::Graph::build(&run.extract).map_err(|m| {
        UseCaseError::Validation(ApiError::new("VALIDATION_ERROR", m, Value::Null))
    })?;
    let mut board = engine::discover(&graph, &run.contract, &run.pack);
    board.snapshot = run.snapshot.clone();
    board.diagnostics = run.diagnostics.clone();

    let experts = load_experts(experts_gw);
    benchmark::match_experts(&mut board.hypotheses, &run.extract.entities, &experts, &run.contract.factory_id);

    run.board = board.clone();
    runs.store(run);

    Ok(board)
}
