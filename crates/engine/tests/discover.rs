//! Интеграционные тесты engine на флотационных фикстурах: golden-портфель +
//! детерминизм + деньги/доступность + generic-операторы (TESTING.md).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use contracts::{
    BoardResponse, Claim, DiagnosticsReport, DomainPack, EdgeType, EvidenceType, ExtractResponse,
    FactoryConfig, GraphEdge, GraphNode, KpiContract, NodeKind, Polarity,
};
use engine::{discover, Graph};
use serde_json::json;

fn data_root() -> PathBuf {
    // Данные лежат в docs/ (единый источник правды).
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../docs")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(data_root().join(rel)).unwrap()
}

/// Аннотировать extract, как это делает платформа: диагноз-узлам —
/// addressable_tons, controllable-рычагам — available.
fn annotate(extract: &mut ExtractResponse, diag: &DiagnosticsReport, factory: &FactoryConfig) {
    let mut by_diag: HashMap<String, HashMap<String, f64>> = HashMap::new();
    for item in &diag.diagnosis_summary {
        *by_diag
            .entry(item.diagnosis.clone())
            .or_default()
            .entry(item.element.clone())
            .or_insert(0.0) += item.tons;
    }
    let present: HashSet<&str> = factory
        .equipment
        .iter()
        .filter(|e| e.present)
        .map(|e| e.id.as_str())
        .collect();
    for node in &mut extract.entities {
        if node.has_tag("diagnosis") {
            if let Some(d) = node.diagnosis_id().map(str::to_string) {
                if let Some(tons) = by_diag.get(&d) {
                    node.properties["addressable_tons"] = json!(tons);
                }
            }
        }
        if node.has_tag("controllable") {
            let available = match node.equipment_required() {
                Some(req) => present.contains(req),
                None => true,
            };
            node.properties["available"] = json!(available);
        }
    }
}

fn load_kgmk() -> (ExtractResponse, DomainPack, KpiContract) {
    let mut extract: ExtractResponse =
        serde_json::from_str(&read("fixtures/extract_response.json")).unwrap();
    let diag: DiagnosticsReport =
        serde_json::from_str(&read("fixtures/diagnostics_kgmk.json")).unwrap();
    let factory: FactoryConfig = serde_yaml::from_str(&read("factories/kgmk.yaml")).unwrap();
    let pack: DomainPack = serde_yaml::from_str(&read("packs/flotation-v1.yaml")).unwrap();
    let board: BoardResponse = serde_json::from_str(&read("fixtures/board.json")).unwrap();
    annotate(&mut extract, &diag, &factory);
    (extract, pack, board.kpi_contract)
}

/// Golden: фикс-граф -> фикс-портфель. Первый прогон генерит файл, далее сверяет.
#[test]
fn golden_board_flotation_v1() {
    let (extract, pack, contract) = load_kgmk();
    let graph = Graph::build(&extract).unwrap();
    let board = discover(&graph, &contract, &pack);
    let actual = serde_json::to_string_pretty(&board).unwrap();

    let golden_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/board_flotation-v1.json");

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
    let (extract, pack, contract) = load_kgmk();
    let graph = Graph::build(&extract).unwrap();
    let a = serde_json::to_string(&discover(&graph, &contract, &pack)).unwrap();
    let b = serde_json::to_string(&discover(&graph, &contract, &pack)).unwrap();
    assert_eq!(a, b);
}

/// Форма портфеля kgmk: ранги 1..N, сортировка по score_total, в топ-3 есть
/// рычаг гидроциклона, есть gap-гипотеза с недоступным рычагом, у денежных
/// гипотез заполнен value_usd_range.
#[test]
fn kgmk_portfolio_shape() {
    let (extract, pack, contract) = load_kgmk();
    let graph = Graph::build(&extract).unwrap();
    let board = discover(&graph, &contract, &pack);

    assert!(!board.hypotheses.is_empty());
    for (i, h) in board.hypotheses.iter().enumerate() {
        assert_eq!(h.rank as usize, i + 1, "ранги должны идти 1..N по порядку");
    }
    for w in board.hypotheses.windows(2) {
        assert!(w[0].score_total >= w[1].score_total, "сортировка по score_total");
    }

    // Топ-3 содержит гипотезу с рычагом гидроциклона.
    let top3_has_hydrocyclone = board.hypotheses.iter().take(3).any(|h| {
        h.source_nodes.iter().any(|n| n == "node_hydrocyclone_nozzle")
    });
    assert!(top3_has_hydrocyclone, "в топ-3 ожидается node_hydrocyclone_nozzle");

    // Гипотеза от gap с недоступным на kgmk рычагом присутствует.
    let has_gap_unavailable = board.hypotheses.iter().any(|h| {
        h.source_nodes.iter().any(|n| {
            n == "node_fine_screening" || n == "node_magnetic_separation"
        })
    });
    assert!(has_gap_unavailable, "ожидается gap-гипотеза с недоступным рычагом");

    // Гипотеза гидроциклона несёт заполненный экономический эффект.
    let hydro = board
        .hypotheses
        .iter()
        .find(|h| h.source_nodes.iter().any(|n| n == "node_hydrocyclone_nozzle"))
        .expect("гипотеза гидроциклона");
    let v = hydro.economic_effect.value_usd_range;
    assert!(v[0] > 0.0 && v[1] > v[0], "value_usd_range заполнен и монотонен");
}

/// change_price: удвоение цены element_28 удваивает value_usd_range денежных гипотез.
#[test]
fn change_price_doubles_value() {
    let (extract, pack, mut contract) = load_kgmk();
    let graph = Graph::build(&extract).unwrap();
    let before = discover(&graph, &contract, &pack);

    let base = before
        .hypotheses
        .iter()
        .find(|h| h.economic_effect.value_usd_range[1] > 0.0)
        .expect("денежная гипотеза")
        .clone();

    let old_price = *contract.prices_usd_per_t.get("element_28").unwrap();
    contract
        .prices_usd_per_t
        .insert("element_28".to_string(), old_price * 2.0);
    let after = discover(&graph, &contract, &pack);

    let same = after
        .hypotheses
        .iter()
        .find(|h| h.title == base.title)
        .expect("та же гипотеза");
    let r0 = same.economic_effect.value_usd_range;
    let r1 = base.economic_effect.value_usd_range;
    assert!((r0[0] - 2.0 * r1[0]).abs() < 1.0);
    assert!((r0[1] - 2.0 * r1[1]).abs() < 1.0);
}

/// excluded_factors обнуляет пути через исключённый узел.
#[test]
fn excluded_factor_changes_portfolio() {
    let (extract, pack, mut contract) = load_kgmk();
    let graph = Graph::build(&extract).unwrap();
    let before = discover(&graph, &contract, &pack);

    contract.excluded_factors.push("node_hydrocyclone_nozzle".to_string());
    let after = discover(&graph, &contract, &pack);

    let mentions = |b: &BoardResponse, id: &str| {
        b.hypotheses.iter().any(|h| h.source_nodes.iter().any(|n| n == id))
    };
    assert!(mentions(&before, "node_hydrocyclone_nozzle"));
    assert!(!mentions(&after, "node_hydrocyclone_nozzle"), "исключённый фактор не должен встречаться");
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
        default_gain_pct_range: [5.0, 15.0],
    };
    let contract = KpiContract {
        factory_id: "synthetic".into(),
        target: contracts::Target {
            metric: "goal".into(),
            direction: "increase".into(),
            minimum_delta_percent: None,
        },
        constraints: vec![],
        prices_usd_per_t: Default::default(),
        weights_override: Default::default(),
        excluded_factors: vec![],
    };

    let graph = Graph::build(&extract).unwrap();
    let board = discover(&graph, &contract, &pack);
    assert!(
        board.hypotheses.iter().any(|h| h.title.contains("граничное условие")),
        "contradiction-гипотеза должна появиться"
    );
}

// ---- helpers для синтетического графа ----

fn claim(id: &str, conf: f64, et: EvidenceType) -> Claim {
    Claim {
        id: id.into(),
        text: id.into(),
        source_ref: "doc".into(),
        source_page: None,
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
