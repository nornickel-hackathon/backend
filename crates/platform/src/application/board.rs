//! Use case `GET /board`: портфель конкретного прогона (`run_id`) или последнего;
//! до первого /run — fallback через `BoardGateway` (fixtures/board.json).

use contracts::BoardResponse;

use crate::application::error::UseCaseError;
use crate::application::ports::{BoardGateway, RunRepository};

pub fn execute(
    runs: &dyn RunRepository,
    board_gateway: &dyn BoardGateway,
    run_id: Option<String>,
) -> Result<BoardResponse, UseCaseError> {
    if let Some(id) = run_id {
        if let Some(run) = runs.get(&id) {
            return Ok(run.board);
        }
    } else if let Some(run) = runs.last() {
        return Ok(run.board);
    }
    board_gateway.load().map_err(UseCaseError::Internal)
}
