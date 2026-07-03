//! Интеграционные тесты engine: golden-портфель на демо-фикстурах + детерминизм
//! + проверка generic-операторов (TESTING.md).

use std::path::PathBuf;

use contracts::{
    BoardResponse, Claim, DomainPack, EdgeType, EvidenceType, ExtractResponse, GraphEdge,
    GraphNode, KpiContract, NodeKind, Polarity, Status,
};
use engine::{discover, Graph};
use serde_json::json;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_demo() -> (ExtractResponse, DomainPack, KpiContract) {
    let root = repo_root();
    let extract: ExtractResponse = serde_json::from_str(
        &std::fs::read_to_string(root.join("fixtures/extract_response.json")).unwrap(),
    )
    .unwrap();
    let pack: DomainPack =
        serde_yaml::from_str(&std::fs::read_to_string(root.join("packs/alloys-v1.yaml")).unwrap())
            .unwrap();
    let board: BoardResponse = serde_json::from_str(
        &std::fs::read_to_string(root.join("fixtures/board.json")).unwrap(),
    )
    .unwrap();
    (extract, pack, board.kpi_contract)
}

/// Golden: фикс-граф -> фикс-портфель. Первый прогон генерит файл, далее сверяет.
#[test]
fn golden_board_alloys_v1() {
    let (extract, pack, contract) = load_demo();
    let graph = Graph::build(&extract).unwrap();
    let board = discover(&graph, &contract, &pack);
    let actual = serde_json::to_string_pretty(&board).unwrap();

    let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/board_alloys-v1.json");

    if std::env::var("UPDATE_GOLDEN").is_ok() || !golden_path.exists() {
        std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        std::fs::write(&golden_path, &actual).unwrap();
    }

    let expected = std::fs::read_to_string(&golden_path).unwrap();
    assert_eq!(actual.trim(), expected.trim(), "engine output drifted from golden");
}

/// Детерминизм: один и тот же вход -> идентичный портфель.
#[test]
fn discover_is_deterministic() {
    let (extract, pack, contract) = load_demo();
    let graph = Graph::build(&extract).unwrap();
    let a = serde_json::to_string(&discover(&graph, &contract, &pack)).unwrap();
    let b = serde_json::to_string(&discover(&graph, &contract, &pack)).unwrap();
    assert_eq!(a, b);
}

/// Базовые свойства портфеля на демо: есть recommended, есть rejected (cost),
/// есть needs_expert_review (непокрытый ductility_loss), ранги 1..N подряд.
#[test]
fn demo_portfolio_shape() {
    let (extract, pack, contract) = load_demo();
    let graph = Graph::build(&extract).unwrap();
    let board = discover(&graph, &contract, &pack);

    assert!(!board.hypotheses.is_empty());
    for (i, h) in board.hypotheses.iter().enumerate() {
        assert_eq!(h.rank as usize, i + 1, "ranks must be 1..N in order");
    }
    // score_total отсортирован по убыванию
    for w in board.hypotheses.windows(2) {
        assert!(w[0].score_total >= w[1].score_total);
    }
    let statuses: Vec<Status> = board.hypotheses.iter().map(|h| h.status).collect();
    assert!(statuses.contains(&Status::RejectedByConstraints), "Sc-путь нарушает cost");
    assert!(
        statuses.contains(&Status::NeedsExpertReview),
        "непокрытый ductility_loss"
    );

    // engine не выдумывает данные: у каждой гипотезы есть DOE-план
    for h in &board.hypotheses {
        assert!(!h.doe_plan.measurements.is_empty());
    }
}

/// excluded_factors обнуляет пути через исключённый узел.
#[test]
fn excluded_factor_changes_portfolio() {
    let (extract, pack, mut contract) = load_demo();
    let graph = Graph::build(&extract).unwrap();
    let before = discover(&graph, &contract, &pack);

    contract.excluded_factors.push("node_sc_addition".to_string());
    let after = discover(&graph, &contract, &pack);

    let mentions_sc = |b: &BoardResponse| {
        b.hypotheses
            .iter()
            .any(|h| h.source_nodes.iter().any(|n| n == "node_sc_addition"))
    };
    assert!(mentions_sc(&before));
    assert!(!mentions_sc(&after), "исключённый фактор не должен встречаться");
}

/// contradiction-оператор включается pack'ом и находит pos/neg на одном узле.
#[test]
fn contradiction_operator_when_enabled() {
    let extract = ExtractResponse {
        pack_id: "synthetic".into(),
        documents: vec![],
        claims: vec![
            claim("c1", 0.9, EvidenceType::Literature),
            claim("c2", 0.8, EvidenceType::Experiment),
        ],
        entities: vec![
            node("f1", NodeKind::Factor, "lever one", &["controllable"]),
            node("f2", NodeKind::Factor, "lever two", &["controllable"]),
            node("k", NodeKind::Kpi, "goal", &["kpi"]),
        ],
        edges: vec![
            edge("e1", "f1", "k", EdgeType::Mechanism, Polarity::Positive, &["c1"]),
            edge("e2", "f2", "k", EdgeType::Mechanism, Polarity::Negative, &["c2"]),
        ],
    };
    let pack = DomainPack {
        pack_id: "synthetic".into(),
        scoring_weights: [("kpi_impact".to_string(), 1.0)].into_iter().collect(),
        hard_constraints: vec![],
        enabled_operators: vec!["mechanism_path".into(), "contradiction".into()],
    };
    let contract = KpiContract {
        target: contracts::Target {
            metric: "goal".into(),
            direction: "increase".into(),
            minimum_delta_percent: None,
        },
        constraints: vec![],
        weights_override: Default::default(),
        excluded_factors: vec![],
    };

    let graph = Graph::build(&extract).unwrap();
    let board = discover(&graph, &contract, &pack);
    assert!(
        board.hypotheses.iter().any(|h| h.title.contains("boundary condition")),
        "contradiction-гипотеза должна появиться"
    );
}

// ---- helpers для синтетического графа ----

fn claim(id: &str, conf: f64, et: EvidenceType) -> Claim {
    Claim {
        id: id.into(),
        text: id.into(),
        source_ref: "doc".into(),
        confidence: conf,
        evidence_type: et,
    }
}

fn node(id: &str, kind: NodeKind, label: &str, tags: &[&str]) -> GraphNode {
    GraphNode {
        id: id.into(),
        kind,
        label: label.into(),
        tags: tags.iter().map(|s| s.to_string()).collect(),
        properties: json!({}),
    }
}

fn edge(
    id: &str,
    src: &str,
    dst: &str,
    edge_type: EdgeType,
    polarity: Polarity,
    claims: &[&str],
) -> GraphEdge {
    GraphEdge {
        id: id.into(),
        src: src.into(),
        dst: dst.into(),
        edge_type,
        mechanism: None,
        source_claims: claims.iter().map(|s| s.to_string()).collect(),
        polarity: Some(polarity),
    }
}
