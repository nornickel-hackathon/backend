//! Use case `POST /rerun`: применить action к контракту и пересчитать БЕЗ
//! extraction. Тот же snapshot/граф — воспроизводимость входа сохраняется.

use contracts::{ApiError, BoardResponse, RerunAction};
use serde_json::Value;

use crate::application::error::UseCaseError;
use crate::application::ports::RunRepository;
use crate::domain::rerun;

pub fn execute(
    runs: &dyn RunRepository,
    action: RerunAction,
) -> Result<BoardResponse, UseCaseError> {
    let mut run = runs
        .last()
        .ok_or_else(|| UseCaseError::NotFound("no run yet; call POST /run first".to_string()))?;

    rerun::apply(&mut run.contract, &action);

    let graph = engine::Graph::build(&run.extract).map_err(|m| {
        UseCaseError::Validation(ApiError::new("VALIDATION_ERROR", m, Value::Null))
    })?;
    let mut board = engine::discover(&graph, &run.contract, &run.pack);
    board.snapshot = run.snapshot.clone();

    run.board = board.clone();
    runs.store(run);

    Ok(board)
}
