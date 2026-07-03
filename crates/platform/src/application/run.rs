//! Use case `POST /run`: загрузить extract, провалидировать, построить граф,
//! прогнать engine, зафиксировать snapshot и сохранить прогон.

use contracts::{ApiError, BoardResponse, KpiContract};
use serde_json::Value;

use crate::application::error::UseCaseError;
use crate::application::ports::{ExtractSource, PackRepository, RunRepository};
use crate::application::run_record::RunRecord;
use crate::domain::{snapshot, validation};

pub struct RunInput {
    pub kpi_contract: KpiContract,
    pub pack_id: Option<String>,
}

pub fn execute(
    extract_source: &dyn ExtractSource,
    packs: &dyn PackRepository,
    runs: &dyn RunRepository,
    input: RunInput,
) -> Result<BoardResponse, UseCaseError> {
    let extract = extract_source.load().map_err(UseCaseError::Internal)?;
    validation::validate(&extract).map_err(UseCaseError::Validation)?;

    let graph = engine::Graph::build(&extract).map_err(|m| {
        UseCaseError::Validation(ApiError::new("VALIDATION_ERROR", m, Value::Null))
    })?;

    let pack_id = input.pack_id.unwrap_or_else(|| extract.pack_id.clone());
    let pack = packs.load(&pack_id).map_err(UseCaseError::Internal)?;

    let mut board = engine::discover(&graph, &input.kpi_contract, &pack);
    let snap = snapshot::snapshot_of(&extract);
    board.snapshot = snap.clone();

    runs.store(RunRecord {
        run_id: format!("run_{}", snap.hash),
        extract,
        snapshot: snap,
        pack,
        contract: input.kpi_contract,
        board: board.clone(),
    });

    Ok(board)
}
