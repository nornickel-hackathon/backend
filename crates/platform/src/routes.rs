//! axum-эндпоинты Rust-платформы (rust_tasks.md P0.8): /run, /board,
//! /hypothesis/:id, /rerun.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use contracts::{ApiError, BoardResponse, ExtractResponse, Hypothesis, KpiContract, RerunAction};
use serde::Deserialize;
use serde_json::Value;

use crate::state::{AppState, RunState};
use crate::{pack, rerun, snapshot, validate};

#[derive(Deserialize)]
pub struct RunRequest {
    pub kpi_contract: KpiContract,
    #[serde(default)]
    pub pack_id: Option<String>,
}

#[derive(Deserialize)]
pub struct BoardQuery {
    #[serde(default)]
    pub run_id: Option<String>,
}

/// Ошибка эндпоинта с HTTP-статусом и телом в формате контракта.
pub struct AppError {
    status: StatusCode,
    body: ApiError,
}

impl AppError {
    fn unprocessable(body: ApiError) -> Self {
        AppError { status: StatusCode::UNPROCESSABLE_ENTITY, body }
    }
    fn internal(message: impl Into<String>) -> Self {
        AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: ApiError::new("INTERNAL_ERROR", message, Value::Null),
        }
    }
    fn not_found(message: impl Into<String>) -> Self {
        AppError {
            status: StatusCode::NOT_FOUND,
            body: ApiError::new("NOT_FOUND", message, Value::Null),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

/// POST /run — построить граф из встроенного extract-фикстуры, прогнать engine.
pub async fn run(
    State(state): State<AppState>,
    Json(req): Json<RunRequest>,
) -> Result<Json<BoardResponse>, AppError> {
    let extract = load_extract(&state)?;
    validate::validate(&extract).map_err(AppError::unprocessable)?;

    let graph = engine::Graph::build(&extract).map_err(|m| {
        AppError::unprocessable(ApiError::new("VALIDATION_ERROR", m, Value::Null))
    })?;

    let pack_id = req.pack_id.clone().unwrap_or_else(|| extract.pack_id.clone());
    let pack = pack::load(&state.base_dir, &pack_id).map_err(AppError::internal)?;

    let mut board = engine::discover(&graph, &req.kpi_contract, &pack);
    let snap = snapshot::snapshot_of(&extract);
    board.snapshot = snap.clone();

    let run_id = format!("run_{}", snap.hash);
    state.store(RunState {
        run_id,
        extract,
        snapshot: snap,
        pack,
        contract: req.kpi_contract,
        board: board.clone(),
    });

    Ok(Json(board))
}

/// GET /board — текущий портфель; до первого /run — fallback на fixtures/board.json.
pub async fn board(
    State(state): State<AppState>,
    Query(_q): Query<BoardQuery>,
) -> Result<Json<BoardResponse>, AppError> {
    if let Some(run) = state.last_run() {
        return Ok(Json(run.board));
    }
    let path = state.fixtures_path("board.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| AppError::internal(format!("cannot read board fixture: {e}")))?;
    let board: BoardResponse = serde_json::from_str(&text)
        .map_err(|e| AppError::internal(format!("cannot parse board fixture: {e}")))?;
    Ok(Json(board))
}

/// GET /hypothesis/:id — одна гипотеза из текущего портфеля.
pub async fn hypothesis(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Hypothesis>, AppError> {
    let run = state
        .last_run()
        .ok_or_else(|| AppError::not_found("no run yet; call POST /run first"))?;
    run.board
        .hypotheses
        .into_iter()
        .find(|h| h.id == id)
        .map(Json)
        .ok_or_else(|| AppError::not_found(format!("hypothesis '{id}' not found")))
}

/// POST /rerun — применить action к контракту и пересчитать БЕЗ extraction.
pub async fn rerun(
    State(state): State<AppState>,
    Json(action): Json<RerunAction>,
) -> Result<Json<BoardResponse>, AppError> {
    let mut run = state
        .last_run()
        .ok_or_else(|| AppError::not_found("no run yet; call POST /run first"))?;

    rerun::apply(&mut run.contract, &action);

    // Тот же snapshot/граф — extraction не повторяется.
    let graph = engine::Graph::build(&run.extract).map_err(|m| {
        AppError::unprocessable(ApiError::new("VALIDATION_ERROR", m, Value::Null))
    })?;
    let mut board = engine::discover(&graph, &run.contract, &run.pack);
    board.snapshot = run.snapshot.clone();

    run.board = board.clone();
    state.store(run);

    Ok(Json(board))
}

fn load_extract(state: &AppState) -> Result<ExtractResponse, AppError> {
    let path = state.fixtures_path("extract_response.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| AppError::internal(format!("cannot read extract fixture: {e}")))?;
    serde_json::from_str(&text)
        .map_err(|e| AppError::internal(format!("cannot parse extract fixture: {e}")))
}
