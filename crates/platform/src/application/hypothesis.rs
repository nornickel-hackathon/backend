//! Use case `GET /hypothesis/:id`: одна гипотеза из текущего портфеля.

use contracts::Hypothesis;

use crate::application::error::UseCaseError;
use crate::application::ports::RunRepository;

pub fn execute(runs: &dyn RunRepository, id: &str) -> Result<Hypothesis, UseCaseError> {
    let run = runs
        .last()
        .ok_or_else(|| UseCaseError::NotFound("no run yet; call POST /run first".to_string()))?;
    run.board
        .hypotheses
        .into_iter()
        .find(|h| h.id == id)
        .ok_or_else(|| UseCaseError::NotFound(format!("hypothesis '{id}' not found")))
}
