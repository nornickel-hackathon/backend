//! axum-хендлеры HTTP-шва: /run, /board, /hypothesis/:id, /rerun, /extract,
//! /expert_hypotheses, /export/board.{json,csv}. Каждый — тонкая обёртка:
//! извлечь вход, вызвать use case, отдать JSON/файл.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use contracts::{ExtractResponse, Hypothesis};
use serde_json::Value;

use crate::application::{self, export, UseCaseError};
use crate::state::AppState;
use crate::web::dto::{BoardQuery, RerunRequest, RunRequest, RunResponse};
use crate::web::error::HttpError;

/// POST /run — граф из extract+диагностики, engine, сохранить прогон → {run_id, board}.
pub async fn run(
    State(state): State<AppState>,
    Json(req): Json<RunRequest>,
) -> Result<Json<RunResponse>, HttpError> {
    let out = application::run::execute(
        state.extract_source.as_ref(),
        state.diagnostics_source.as_ref(),
        state.factories.as_ref(),
        state.packs.as_ref(),
        state.expert_hypotheses.as_ref(),
        state.runs.as_ref(),
        application::run::RunInput {
            factory_id: req.factory_id,
            pack_id: req.pack_id,
            kpi_contract: req.kpi_contract,
        },
    )?;
    Ok(Json(RunResponse {
        run_id: out.run_id,
        board: out.board,
    }))
}

/// GET /board — портфель прогона (run_id) или последнего; иначе fallback fixtures.
pub async fn board(
    State(state): State<AppState>,
    Query(q): Query<BoardQuery>,
) -> Result<Json<contracts::BoardResponse>, HttpError> {
    let board =
        application::board::execute(state.runs.as_ref(), state.board_gateway.as_ref(), q.run_id)?;
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

/// POST /rerun — применить action к контракту прогона и пересчитать БЕЗ extraction.
pub async fn rerun(
    State(state): State<AppState>,
    Json(req): Json<RerunRequest>,
) -> Result<Json<contracts::BoardResponse>, HttpError> {
    let board = application::rerun::execute(
        state.runs.as_ref(),
        state.expert_hypotheses.as_ref(),
        req.run_id,
        req.action,
    )?;
    Ok(Json(board))
}

/// GET /benchmark?run_id= — покрытие эталонных гипотез экспертов (golden set).
pub async fn benchmark(
    State(state): State<AppState>,
    Query(q): Query<BoardQuery>,
) -> Result<Json<contracts::BenchmarkReport>, HttpError> {
    let report =
        application::benchmark::execute(state.runs.as_ref(), state.expert_hypotheses.as_ref(), q.run_id)?;
    Ok(Json(report))
}

/// GET /data_readiness?run_id= — качество исходных данных прогона.
pub async fn data_readiness(
    State(state): State<AppState>,
    Query(q): Query<BoardQuery>,
) -> Result<Json<contracts::DataReadiness>, HttpError> {
    let report = application::readiness::execute(state.runs.as_ref(), q.run_id)?;
    Ok(Json(report))
}

/// GET /trace/:id?run_id= — трассировка гипотезы до claims и ячеек xlsx.
pub async fn trace(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<BoardQuery>,
) -> Result<Json<contracts::TraceReport>, HttpError> {
    let report = application::trace::execute(state.runs.as_ref(), q.run_id, &id)?;
    Ok(Json(report))
}

/// GET /factories — мультифабричная карта денег (все фабрики кейса).
pub async fn factories(
    State(state): State<AppState>,
) -> Result<Json<Vec<contracts::FactorySummary>>, HttpError> {
    let summaries = application::factories::execute(
        state.extract_source.as_ref(),
        state.diagnostics_source.as_ref(),
        state.factories.as_ref(),
        state.packs.as_ref(),
        state.expert_hypotheses.as_ref(),
    )?;
    Ok(Json(summaries))
}

/// GET /extract — текущий ExtractResponse (документы + claims) для Library/trace.
pub async fn extract(
    State(state): State<AppState>,
) -> Result<Json<ExtractResponse>, HttpError> {
    let extract = state
        .extract_source
        .load()
        .map_err(UseCaseError::Internal)?;
    Ok(Json(extract))
}

/// GET /expert_hypotheses — golden/expert_hypotheses.json для Benchmark view.
pub async fn expert_hypotheses(
    State(state): State<AppState>,
) -> Result<Json<Value>, HttpError> {
    let data = state
        .expert_hypotheses
        .load()
        .map_err(UseCaseError::Internal)?;
    Ok(Json(data))
}

/// GET /export/board.json — текущий BoardResponse как attachment.
pub async fn export_board_json(
    State(state): State<AppState>,
) -> Result<Response, HttpError> {
    let board =
        application::board::execute(state.runs.as_ref(), state.board_gateway.as_ref(), None)?;
    let body = serde_json::to_string_pretty(&board)
        .map_err(|e| UseCaseError::Internal(e.to_string()))?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"board.json\"",
            ),
        ],
        body,
    )
        .into_response())
}

/// GET /export/board.csv — плоская выгрузка гипотез как attachment.
pub async fn export_board_csv(
    State(state): State<AppState>,
) -> Result<Response, HttpError> {
    let board =
        application::board::execute(state.runs.as_ref(), state.board_gateway.as_ref(), None)?;
    let csv = export::board_csv(&board);
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"board.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}
