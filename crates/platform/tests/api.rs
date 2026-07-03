//! Контрактные тесты axum-API (TESTING.md): /run, /board, /rerun, валидация входа.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use contracts::BoardResponse;
use http_body_util::BodyExt;
use platform::state::AppState;
use platform::{build_router, validate};
use serde_json::{json, Value};
use tower::ServiceExt;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_request_body() -> Value {
    let board: Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("fixtures/board.json")).unwrap(),
    )
    .unwrap();
    json!({ "kpi_contract": board["kpi_contract"], "pack_id": "alloys-v1" })
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn app() -> axum::Router {
    build_router(AppState::new(repo_root()))
}

async fn post(app: &axum::Router, path: &str, body: Value) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn run_returns_valid_board() {
    let app = app();
    let resp = post(&app, "/run", run_request_body()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("X-Contract-Version").unwrap(),
        "1"
    );
    let board: BoardResponse = serde_json::from_value(body_json(resp).await).unwrap();
    assert!(!board.hypotheses.is_empty());
    assert_eq!(board.snapshot.pack_id, "alloys-v1");
}

#[tokio::test]
async fn board_reflects_last_run() {
    let app = app();
    let run_resp = post(&app, "/run", run_request_body()).await;
    let run_board: BoardResponse = serde_json::from_value(body_json(run_resp).await).unwrap();

    let get_resp = app
        .clone()
        .oneshot(Request::builder().uri("/board").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_board: BoardResponse = serde_json::from_value(body_json(get_resp).await).unwrap();
    assert_eq!(run_board.snapshot.hash, get_board.snapshot.hash);
    assert_eq!(run_board.hypotheses.len(), get_board.hypotheses.len());
}

#[tokio::test]
async fn rerun_changes_ranking_keeps_snapshot() {
    let app = app();
    let run_board: BoardResponse =
        serde_json::from_value(body_json(post(&app, "/run", run_request_body()).await).await)
            .unwrap();

    let action = json!({ "kind": "exclude_factor", "payload": { "factor_id": "node_sc_addition" } });
    let rerun_resp = post(&app, "/rerun", action).await;
    assert_eq!(rerun_resp.status(), StatusCode::OK);
    let rerun_board: BoardResponse =
        serde_json::from_value(body_json(rerun_resp).await).unwrap();

    // snapshot тот же — extraction не повторялся
    assert_eq!(run_board.snapshot.hash, rerun_board.snapshot.hash);
    // исключённый фактор исчез из портфеля
    assert!(rerun_board
        .hypotheses
        .iter()
        .all(|h| h.source_nodes.iter().all(|n| n != "node_sc_addition")));
    // ранжирование изменилось
    let before: Vec<&str> = run_board.hypotheses.iter().map(|h| h.title.as_str()).collect();
    let after: Vec<&str> = rerun_board.hypotheses.iter().map(|h| h.title.as_str()).collect();
    assert_ne!(before, after);
}

#[tokio::test]
async fn rerun_without_run_is_404() {
    let app = app();
    let action = json!({ "kind": "exclude_factor", "payload": { "factor_id": "x" } });
    let resp = post(&app, "/rerun", action).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Валидация входного JSON: висящая ссылка ребра -> 422-ошибка контракта.
#[test]
fn validate_rejects_dangling_edge() {
    let extract: contracts::ExtractResponse = serde_json::from_value(json!({
        "pack_id": "alloys-v1",
        "entities": [
            { "id": "n1", "kind": "factor", "label": "a", "tags": ["controllable"] }
        ],
        "edges": [
            { "id": "e1", "src": "n1", "dst": "ghost", "edge_type": "mechanism" }
        ],
        "claims": []
    }))
    .unwrap();

    let err = validate::validate(&extract).unwrap_err();
    assert_eq!(err.error.code, "VALIDATION_ERROR");
    assert_eq!(err.error.details["missing_node"], "ghost");
}
