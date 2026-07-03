//! Scoring по docs/SCORING.md. Все формулы — generic; `cost` здесь это
//! первоклассное измерение контракта (`ScoreBreakdown.cost`), а не доменное слово.

use std::collections::HashMap;

use contracts::{DomainPack, KpiContract, ScoreBreakdown};

use crate::graph::Graph;
use crate::operators::Candidate;

/// Свойство-узла с оценкой относительного роста стоимости (см. CONTRACTS.md).
const COST_PROPERTY: &str = "estimated_cost_delta_percent";
/// Метрика-измерение стоимости в контракте/паке.
const COST_METRIC: &str = "cost";

pub struct Scored {
    pub breakdown: ScoreBreakdown,
    pub risks: Vec<String>,
    pub missing_evidence: Vec<String>,
}

pub fn score(
    graph: &Graph,
    cand: &Candidate,
    contract: &KpiContract,
    pack: &DomainPack,
) -> Scored {
    let missing_evidence = missing_evidence(graph, contract);

    let kpi_impact = {
        let path_claims = graph.claims_on_edges(&cand.edges);
        if path_claims.is_empty() {
            0.0
        } else {
            let penalty = 0.05 * ((cand.source_nodes.len() as f64) - 2.0).max(0.0);
            (mean_confidence(graph, &path_claims) * (1.0 - penalty)).clamp(0.0, 1.0)
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

    let cost = cost_score(graph, cand, contract, pack);

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
        risks.push("Estimated cost delta may exceed the constraint.".to_string());
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
    overrides: &HashMap<String, f64>,
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

fn missing_evidence(graph: &Graph, contract: &KpiContract) -> Vec<String> {
    let mut out = Vec::new();
    for con in &contract.constraints {
        if !metric_has_support(graph, &con.metric) {
            out.push(format!("No claim covers constraint metric '{}'.", con.metric));
        }
    }
    out
}

/// Метрика покрыта, если есть constraint-узел, чей label/id соответствует метрике,
/// и у него есть инцидентное ребро с claim'ами.
fn metric_has_support(graph: &Graph, metric: &str) -> bool {
    for idx in graph.nodes_with_tag("constraint") {
        let node = graph.weight(idx);
        if normalize(&node.label) == normalize(metric) || normalize(&node.id).contains(&normalize(metric)) {
            return graph.node_has_evidenced_edge(idx);
        }
    }
    false
}

fn cost_score(
    graph: &Graph,
    cand: &Candidate,
    contract: &KpiContract,
    pack: &DomainPack,
) -> f64 {
    let delta = cand
        .controllable
        .as_ref()
        .and_then(|id| graph.node(id))
        .and_then(|n| n.properties.get(COST_PROPERTY).and_then(serde_json::Value::as_f64));
    let limit = cost_limit(contract, pack);
    match (delta, limit) {
        (Some(d), Some(l)) if d > l => l / d,
        (Some(_), _) => 1.0,
        (None, _) => 0.7,
    }
}

/// Лимит cost-метрики: сначала из контракта, затем из hard_constraints пака.
pub fn cost_limit(contract: &KpiContract, pack: &DomainPack) -> Option<f64> {
    contract
        .constraints
        .iter()
        .find(|c| normalize(&c.metric) == COST_METRIC)
        .map(|c| c.value)
        .or_else(|| {
            pack.hard_constraints
                .iter()
                .find(|c| normalize(&c.metric) == COST_METRIC)
                .map(|c| c.value)
        })
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
