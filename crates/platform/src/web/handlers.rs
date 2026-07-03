//! axum-хендлеры (rust_tasks.md P0.8): /run, /board, /hypothesis/:id, /rerun.
//! Каждый — тонкая обёртка: извлечь вход, вызвать use case, отдать JSON.

use axum::extract::{Path, Query, State};
use axum::Json;
use contracts::{BoardResponse, Hypothesis, RerunAction};

use crate::application;
use crate::state::AppState;
use crate::web::dto::{BoardQuery, RunRequest};
use crate::web::error::HttpError;

/// POST /run — построить граф из extract, прогнать engine, сохранить прогон.
pub async fn run(
    State(state): State<AppState>,
    Json(req): Json<RunRequest>,
) -> Result<Json<BoardResponse>, HttpError> {
    let board = application::run::execute(
        state.extract_source.as_ref(),
        state.packs.as_ref(),
        state.runs.as_ref(),
        application::run::RunInput {
            kpi_contract: req.kpi_contract,
            pack_id: req.pack_id,
        },
    )?;
    Ok(Json(board))
}

/// GET /board — текущий портфель; до первого /run — fallback на fixtures/board.json.
pub async fn board(
    State(state): State<AppState>,
    Query(_q): Query<BoardQuery>,
) -> Result<Json<BoardResponse>, HttpError> {
    let board = application::board::execute(state.runs.as_ref(), state.board_gateway.as_ref())?;
    Ok(Json(board))
}

/// GET /hypothesis/:id — одна гипотеза из текущего портфеля.
pub async fn hypothesis(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Hypothesis>, HttpError> {
    let hyp = application::hypothesis::execute(state.runs.as_ref(), &id)?;
    Ok(Json(hyp))
}

/// POST /rerun — применить action к контракту и пересчитать БЕЗ extraction.
pub async fn rerun(
    State(state): State<AppState>,
    Json(action): Json<RerunAction>,
) -> Result<Json<BoardResponse>, HttpError> {
    let board = application::rerun::execute(state.runs.as_ref(), action)?;
    Ok(Json(board))
}
