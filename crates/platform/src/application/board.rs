//! Use case `GET /board`: текущий портфель последнего прогона; до первого
//! /run — fallback через `BoardGateway`.

use contracts::BoardResponse;

use crate::application::error::UseCaseError;
use crate::application::ports::{BoardGateway, RunRepository};

pub fn execute(
    runs: &dyn RunRepository,
    board_gateway: &dyn BoardGateway,
) -> Result<BoardResponse, UseCaseError> {
    if let Some(run) = runs.last() {
        return Ok(run.board);
    }
    board_gateway.load().map_err(UseCaseError::Internal)
}
