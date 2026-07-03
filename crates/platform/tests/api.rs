//! Контрактные тесты axum-API (TESTING.md): /run, /board, /rerun, export,
//! десериализация флотационных фикстур, валидация входа.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use contracts::{BoardResponse, DiagnosticsReport};
use http_body_util::BodyExt;
use platform::state::AppState;
use platform::{build_router, validate};
use serde_json::{json, Value};
use tower::ServiceExt;

fn data_root() -> PathBuf {
    // Единый источник данных — docs/.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../docs")
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn body_text(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn app() -> axum::Router {
    build_router(AppState::new(data_root()))
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

async fn get(app: &axum::Router, path: &str) -> axum::response::Response {
    app.clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// POST /run по factory_id (без явного контракта) -> {run_id, board}; в топ-3
/// гипотеза гидроциклона с value_usd_range ≈ [3.96M, 11.87M]; есть диагностика.
#[tokio::test]
async fn run_returns_wrapped_board_with_money() {
    let app = app();
    let resp = post(&app, "/run", json!({ "factory_id": "kgmk" })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("X-Contract-Version").unwrap(), "1");

    let body = body_json(resp).await;
    assert!(body["run_id"].as_str().unwrap().starts_with("run_"));
    let board: BoardResponse = serde_json::from_value(body["board"].clone()).unwrap();
    assert_eq!(board.snapshot.pack_id, "flotation-v1");
    assert_eq!(board.diagnostics.factory_id, "kgmk");
    assert!(!board.diagnostics.loss_cells.is_empty());

    let hydro = board
        .hypotheses
        .iter()
        .find(|h| h.source_nodes.iter().any(|n| n == "node_hydrocyclone_nozzle"))
        .expect("гипотеза гидроциклона");
    assert!(hydro.rank <= 3, "гидроциклон ожидается в топ-3");
    let v = hydro.economic_effect.value_usd_range;
    assert!((v[0] - 3_955_050.0).abs() < 1.0, "value_lo ≈ 3.96M, got {}", v[0]);
    assert!((v[1] - 11_865_150.0).abs() < 1.0, "value_hi ≈ 11.87M, got {}", v[1]);

    // gap-гипотеза с недоступным на kgmk рычагом присутствует.
    assert!(board.hypotheses.iter().any(|h| {
        h.source_nodes
            .iter()
            .any(|n| n == "node_fine_screening" || n == "node_magnetic_separation")
    }));
}

#[tokio::test]
async fn board_reflects_last_run() {
    let app = app();
    let run_body = body_json(post(&app, "/run", json!({ "factory_id": "kgmk" })).await).await;
    let run_board: BoardResponse = serde_json::from_value(run_body["board"].clone()).unwrap();

    let get_board: BoardResponse =
        serde_json::from_value(body_json(get(&app, "/board").await).await).unwrap();
    assert_eq!(run_board.snapshot.hash, get_board.snapshot.hash);
    assert_eq!(run_board.hypotheses.len(), get_board.hypotheses.len());
}

/// change_price element_28 ×2 удваивает value_usd_range; snapshot.hash не меняется.
#[tokio::test]
async fn rerun_change_price_doubles_value_keeps_snapshot() {
    let app = app();
    let run_body = body_json(post(&app, "/run", json!({ "factory_id": "kgmk" })).await).await;
    let run_id = run_body["run_id"].as_str().unwrap().to_string();
    let run_board: BoardResponse = serde_json::from_value(run_body["board"].clone()).unwrap();
    let top = &run_board.hypotheses[0];
    let before = top.economic_effect.value_usd_range;

    let action = json!({
        "run_id": run_id,
        "action": { "kind": "change_price", "payload": { "element": "element_28", "usd_per_t": 33000 } }
    });
    let rerun_resp = post(&app, "/rerun", action).await;
    assert_eq!(rerun_resp.status(), StatusCode::OK);
    let rerun_board: BoardResponse = serde_json::from_value(body_json(rerun_resp).await).unwrap();

    assert_eq!(run_board.snapshot.hash, rerun_board.snapshot.hash);
    let after = rerun_board
        .hypotheses
        .iter()
        .find(|h| h.title == top.title)
        .unwrap()
        .economic_effect
        .value_usd_range;
    assert!((after[0] - 2.0 * before[0]).abs() < 1.0);
    assert!((after[1] - 2.0 * before[1]).abs() < 1.0);
}

#[tokio::test]
async fn rerun_without_run_is_404() {
    let app = app();
    let action = json!({ "action": { "kind": "exclude_factor", "payload": { "factor_id": "x" } } });
    let resp = post(&app, "/rerun", action).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn export_board_csv_is_valid() {
    let app = app();
    post(&app, "/run", json!({ "factory_id": "kgmk" })).await;
    let resp = get(&app, "/export/board.csv").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let csv = body_text(resp).await;
    let mut lines = csv.lines();
    assert_eq!(
        lines.next().unwrap(),
        "rank,id,title,status,score_total,value_usd_lo,value_usd_hi,capex_class,addressable_tons_28,trace"
    );
    assert!(lines.next().is_some(), "должна быть хотя бы одна строка данных");
}

#[tokio::test]
async fn extract_and_expert_hypotheses_available() {
    let app = app();
    let extract = body_json(get(&app, "/extract").await).await;
    assert_eq!(extract["pack_id"], "flotation-v1");
    let experts = body_json(get(&app, "/expert_hypotheses").await).await;
    assert!(experts.as_array().unwrap().iter().any(|e| e["id"] == "kgmk_h3"));
}

/// Все 4 diagnostics_*.json десериализуются в DiagnosticsReport (DoD).
#[test]
fn all_diagnostics_deserialize() {
    for f in ["kgmk", "nof_vkr", "nof_med", "tof"] {
        let text =
            std::fs::read_to_string(data_root().join(format!("fixtures/diagnostics_{f}.json")))
                .unwrap();
        let d: DiagnosticsReport = serde_json::from_str(&text).unwrap();
        assert_eq!(d.factory_id, f);
        assert!(!d.diagnosis_summary.is_empty(), "{f}: непустой diagnosis_summary");
    }
}

/// fixtures/board.json десериализуется в BoardResponse (DoD).
#[test]
fn board_fixture_deserializes() {
    let text = std::fs::read_to_string(data_root().join("fixtures/board.json")).unwrap();
    let board: BoardResponse = serde_json::from_str(&text).unwrap();
    assert_eq!(board.kpi_contract.factory_id, "kgmk");
    assert!(!board.hypotheses.is_empty());
    assert!(board.hypotheses[0].economic_effect.value_usd_range[1] > 0.0);
}

/// GET /benchmark: engine переоткрывает все 5 эталонных гипотез экспертов KGMK.
#[tokio::test]
async fn benchmark_covers_kgmk_experts() {
    let app = app();
    post(&app, "/run", json!({ "factory_id": "kgmk" })).await;
    let r = body_json(get(&app, "/benchmark").await).await;
    assert_eq!(r["factory_id"], "kgmk");
    assert_eq!(r["expert_total"], 5);
    assert_eq!(r["matched"], 5, "все эталонные гипотезы KGMK переоткрыты");
    assert!((r["coverage_pct"].as_f64().unwrap() - 100.0).abs() < 1e-6);
}

/// GET /data_readiness: репортит распарсенные ячейки и обработанные ref_error.
#[tokio::test]
async fn data_readiness_reports_quality() {
    let app = app();
    post(&app, "/run", json!({ "factory_id": "kgmk" })).await;
    let r = body_json(get(&app, "/data_readiness").await).await;
    assert!(r["loss_cells"].as_u64().unwrap() > 0);
    assert!(r["issues_by_type"]["ref_error"].as_u64().unwrap() > 0);
    let pct = r["readiness_pct"].as_f64().unwrap();
    assert!((0.0..=100.0).contains(&pct));
}

/// GET /trace/:id: гипотеза резолвится в claims (со страницами) и ячейки xlsx.
#[tokio::test]
async fn trace_resolves_claims_and_cells() {
    let app = app();
    post(&app, "/run", json!({ "factory_id": "kgmk" })).await;
    let r = body_json(get(&app, "/trace/hyp_001").await).await;
    assert!(!r["claims"].as_array().unwrap().is_empty());
    let cells = r["source_cells"].as_array().unwrap();
    assert!(!cells.is_empty());
    assert!(cells[0]["cell_ref"].as_str().unwrap().contains('!'), "cell_ref вида Лист!Ячейка");
}

/// GET /factories: карта денег по всем 4 фабрикам кейса.
#[tokio::test]
async fn factories_money_map() {
    let app = app();
    let arr = body_json(get(&app, "/factories").await).await;
    let list = arr.as_array().unwrap();
    assert_eq!(list.len(), 4);
    let kgmk = list.iter().find(|f| f["factory_id"] == "kgmk").unwrap();
    assert!(kgmk["opportunity_usd_mid"].as_f64().unwrap() > 0.0);
    assert!(kgmk["n_hypotheses"].as_u64().unwrap() > 0);
}

/// GET /roadmap: честная де-дубликация — total по диагнозам, не по всем гипотезам.
#[tokio::test]
async fn roadmap_dedupes_value_by_diagnosis() {
    let app = app();
    post(&app, "/run", json!({ "factory_id": "kgmk" })).await;

    let full = body_json(get(&app, "/roadmap").await).await;
    let total_hi = full["total_value_usd_range"][1].as_f64().unwrap();
    assert!(full["covered_diagnoses"].as_u64().unwrap() >= 1);

    // Сумма по всем гипотезам портфеля (наивная, с двойным счётом) должна быть
    // существенно больше честного total дорожной карты.
    let board = body_json(get(&app, "/board").await).await;
    let naive_hi: f64 = board["hypotheses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["economic_effect"]["value_usd_range"][1].as_f64().unwrap())
        .sum();
    assert!(naive_hi > total_hi * 1.5, "roadmap должен убирать двойной счёт: naive={naive_hi} total={total_hi}");

    // Бюджет max_capex=1 (только быстрые настройки) не даёт больше, чем полный план.
    let cheap = body_json(get(&app, "/roadmap?max_capex=1").await).await;
    assert_eq!(cheap["max_capex_class"], 1);
    for ph in cheap["phases"].as_array().unwrap() {
        assert_eq!(ph["capex_class"], 1);
    }
}

/// Валидация входного JSON: висящая ссылка ребра -> 422-ошибка контракта.
#[test]
fn validate_rejects_dangling_edge() {
    let extract: contracts::ExtractResponse = serde_json::from_value(json!({
        "pack_id": "flotation-v1",
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
