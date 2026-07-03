//! Scoring по docs/SCORING.md. Все формулы — generic; `cost` здесь это
//! первоклассное измерение контракта (`ScoreBreakdown.cost`), а не доменное слово.
//! `kpi_impact` считается от денег (addressable_tons × gain × price), нормируется
//! на максимум портфеля вызывающей стороной (двухпроходно — см. lib.rs).

use std::collections::BTreeMap;

use contracts::{DomainPack, EconomicEffect, KpiContract, ScoreBreakdown};

use crate::graph::Graph;
use crate::operators::Candidate;

pub struct Scored {
    pub breakdown: ScoreBreakdown,
    pub risks: Vec<String>,
    pub missing_evidence: Vec<String>,
}

/// Экономический эффект гипотезы: value_usd_range из addressable_tons диагноз-узла,
/// диапазона прироста извлечения пака и цен контракта. Каждое число объяснимо
/// через `assumptions`. Кандидат без diagnosis_node -> пустой эффект (value = 0).
pub fn economic(
    graph: &Graph,
    cand: &Candidate,
    contract: &KpiContract,
    pack: &DomainPack,
) -> EconomicEffect {
    let [lo, hi] = pack.default_gain_pct_range;

    let Some(diag_id) = &cand.diagnosis_node else {
        return EconomicEffect {
            addressable_tons: BTreeMap::new(),
            recovery_gain_pct_range: [lo, hi],
            value_usd_range: [0.0, 0.0],
            assumptions: Vec::new(),
        };
    };
    let Some(diag_node) = graph.node(diag_id) else {
        return EconomicEffect {
            addressable_tons: BTreeMap::new(),
            recovery_gain_pct_range: [lo, hi],
            value_usd_range: [0.0, 0.0],
            assumptions: Vec::new(),
        };
    };

    let tons_by_el = diag_node.addressable_tons();
    let elements = relevant_elements(graph, cand, contract, &tons_by_el);

    let mut addressable = BTreeMap::new();
    let (mut v_lo, mut v_hi) = (0.0_f64, 0.0_f64);
    let mut assumptions = Vec::new();
    for el in &elements {
        let tons = tons_by_el.get(el).copied().unwrap_or(0.0);
        let price = contract.prices_usd_per_t.get(el).copied().unwrap_or(0.0);
        addressable.insert(el.clone(), tons);
        v_lo += tons * lo / 100.0 * price;
        v_hi += tons * hi / 100.0 * price;
        assumptions.push(format!("price {el} = {price} $/t (параметр KpiContract)"));
    }
    if let Some(diag) = diag_node.diagnosis_id() {
        assumptions.push(format!(
            "диагноз {diag}: addressable_tons из DiagnosticsReport"
        ));
    }
    assumptions.push(format!(
        "gain range {lo}-{hi}% — default_gain_pct_range пака"
    ));

    EconomicEffect {
        addressable_tons: addressable,
        recovery_gain_pct_range: [lo, hi],
        value_usd_range: [v_lo, v_hi],
        assumptions,
    }
}

/// Средняя точка value_usd_range — это value при mid(gain) (линейно по gain).
pub fn value_mid(ee: &EconomicEffect) -> f64 {
    (ee.value_usd_range[0] + ee.value_usd_range[1]) / 2.0
}

/// Элементы, релевантные KPI кандидата: ключи цен, встречающиеся в id KPI-узла.
/// Fallback — все элементы из addressable_tons диагноз-узла.
fn relevant_elements(
    graph: &Graph,
    cand: &Candidate,
    contract: &KpiContract,
    tons_by_el: &BTreeMap<String, f64>,
) -> Vec<String> {
    let kpi_id = graph.node(&cand.kpi).map(|n| normalize(&n.id)).unwrap_or_default();
    let mut els: Vec<String> = contract
        .prices_usd_per_t
        .keys()
        .filter(|k| kpi_id.contains(&normalize(k)))
        .cloned()
        .collect();
    if els.is_empty() {
        els = tons_by_el.keys().cloned().collect();
    }
    els.sort();
    els
}

pub fn score(
    graph: &Graph,
    cand: &Candidate,
    contract: &KpiContract,
    value_mid: f64,
    max_value_mid: f64,
) -> Scored {
    let missing_evidence = missing_evidence(graph, contract);

    // kpi_impact ∝ деньги, нормировано на максимум портфеля.
    let kpi_impact = {
        let path_claims = graph.claims_on_edges(&cand.edges);
        if value_mid <= 0.0 || max_value_mid <= 0.0 || path_claims.is_empty() {
            0.0
        } else {
            let penalty = 0.05 * ((cand.source_nodes.len() as f64) - 2.0).max(0.0);
            ((value_mid / max_value_mid) * mean_confidence(graph, &path_claims) * (1.0 - penalty))
                .clamp(0.0, 1.0)
        }
    };

    let evidence = if cand.trace.is_empty() {
        0.0
    } else {
        mean_confidence(graph, &cand.trace) * (cand.trace.len() as f64 / 3.0).min(1.0)
    };

    let plausibility = {
        let total = cand.edges.len();
        if total == 0 {
            0.0
        } else {
            let grounded = cand.edges.iter().filter(|e| edge_grounded(graph, **e)).count();
            grounded as f64 / total as f64
        }
    };

    let cost = cost_score(graph, cand);

    let n_contradictions = if cand.operator == "contradiction" { 1 } else { 0 };
    let risk = (1.0
        - 0.1 * missing_evidence.len() as f64
        - 0.15 * n_contradictions as f64)
        .clamp(0.0, 1.0);

    let novelty = if cand.is_gap {
        (0.8 - 0.1 * cand.n_partial_matches as f64).clamp(0.0, 1.0)
    } else {
        0.3
    };

    let mut risks = Vec::new();
    if cost < 1.0 {
        risks.push("Higher capex class increases capital cost.".to_string());
    }
    if plausibility < 1.0 && !cand.edges.is_empty() {
        risks.push("Some causal edges rely on non-grounded evidence.".to_string());
    }
    if !missing_evidence.is_empty() {
        risks.push("Some constraints are not directly evidenced.".to_string());
    }

    Scored {
        breakdown: ScoreBreakdown {
            kpi_impact,
            evidence,
            plausibility,
            cost,
            risk,
            novelty,
        },
        risks,
        missing_evidence,
    }
}

/// score_total = Σ weight[dim] * score[dim], веса из pack + override (без нормализации).
pub fn weighted_total(
    b: &ScoreBreakdown,
    pack: &DomainPack,
    overrides: &BTreeMap<String, f64>,
) -> f64 {
    let dims = [
        ("kpi_impact", b.kpi_impact),
        ("evidence", b.evidence),
        ("plausibility", b.plausibility),
        ("cost", b.cost),
        ("risk", b.risk),
        ("novelty", b.novelty),
    ];
    dims.iter().map(|(k, v)| pack.weight(k, overrides) * v).sum()
}

/// Флагаем только те constraint-метрики, у которых есть узел-ограничение в графе,
/// но нет ни одного claim (числовые constraints вроде capex_class — не флагаем).
fn missing_evidence(graph: &Graph, contract: &KpiContract) -> Vec<String> {
    let mut out = Vec::new();
    for con in &contract.constraints {
        if let Some(idx) = constraint_node(graph, &con.metric) {
            if !graph.node_has_evidenced_edge(idx) {
                out.push(format!("No claim covers constraint metric '{}'.", con.metric));
            }
        }
    }
    out
}

/// Constraint-узел, чей label/id соответствует метрике.
fn constraint_node(graph: &Graph, metric: &str) -> Option<petgraph::graph::NodeIndex> {
    for idx in graph.nodes_with_tag("constraint") {
        let node = graph.weight(idx);
        if normalize(&node.label) == normalize(metric)
            || normalize(&node.id).contains(&normalize(metric))
        {
            return Some(idx);
        }
    }
    None
}

/// cost ∈ {1: 1.0, 2: 0.7, 3: 0.35}[capex_class]; неизвестно -> 0.7.
fn cost_score(graph: &Graph, cand: &Candidate) -> f64 {
    let capex = cand
        .controllable
        .as_ref()
        .and_then(|id| graph.node(id))
        .and_then(|n| n.capex_class());
    match capex {
        Some(1) => 1.0,
        Some(2) => 0.7,
        Some(3) => 0.35,
        _ => 0.7,
    }
}

fn edge_grounded(graph: &Graph, edge: petgraph::graph::EdgeIndex) -> bool {
    graph
        .edge(edge)
        .source_claims
        .iter()
        .filter_map(|id| graph.claim(id))
        .any(|c| c.evidence_type.is_grounded())
}

fn mean_confidence(graph: &Graph, claim_ids: &[String]) -> f64 {
    let confs: Vec<f64> = claim_ids
        .iter()
        .filter_map(|id| graph.claim(id))
        .map(|c| c.confidence)
        .collect();
    if confs.is_empty() {
        0.0
    } else {
        confs.iter().sum::<f64>() / confs.len() as f64
    }
}

fn normalize(s: &str) -> String {
    s.to_lowercase().replace([' ', '_', '-'], "")
}
