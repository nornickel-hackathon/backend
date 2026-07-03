//! crates/engine — Discovery Engine (Роль 1).
//!
//! ЯДРО. Чистый Rust, без I/O, без HTTP, без доменных слов (AGENT_RULES.md §1).
//! Единственная публичная функция — [`discover`]. Детерминизм обязателен: один и
//! тот же граф+контракт+pack обязаны давать один и тот же портфель.

mod doe;
mod graph;
mod operators;
mod scoring;
mod status;

use std::cmp::Ordering;
use std::collections::BTreeSet;

use contracts::{BoardResponse, DomainPack, Hypothesis, KpiContract, Snapshot};

pub use graph::Graph;

/// Обойти граф generic-операторами, посчитать scoring/статусы и вернуть
/// ранжированный портфель гипотез.
///
/// `snapshot.id`/`snapshot.hash` остаются пустыми — их проставляет платформа,
/// которая владеет воспроизводимостью входа.
pub fn discover(graph: &Graph, contract: &KpiContract, pack: &DomainPack) -> BoardResponse {
    let candidates = operators::generate(graph, contract, pack);

    let mut seen = BTreeSet::new();
    let mut scored: Vec<(f64, String, Hypothesis)> = Vec::new();

    for cand in candidates {
        let key = format!("{}|{}", cand.operator, cand.source_nodes.join(","));
        if !seen.insert(key.clone()) {
            continue;
        }

        let s = scoring::score(graph, &cand, contract, pack);
        let score_total = scoring::weighted_total(&s.breakdown, pack, &contract.weights_override);
        let st = status::assign(graph, &cand, score_total, contract, pack);
        let doe_plan = doe::plan(graph, &cand);

        let hyp = Hypothesis {
            id: String::new(),
            title: title(graph, &cand),
            summary: summary(graph, &cand),
            status: st,
            rank: 0,
            score_total,
            score_breakdown: s.breakdown,
            trace: cand.trace.clone(),
            source_nodes: cand.source_nodes.clone(),
            risks: s.risks,
            missing_evidence: s.missing_evidence,
            doe_plan,
        };
        scored.push((score_total, key, hyp));
    }

    // Сортировка: score_total desc, тай-брейк по стабильному ключу.
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal).then_with(|| a.1.cmp(&b.1))
    });

    let hypotheses = scored
        .into_iter()
        .enumerate()
        .map(|(i, (_, _, mut h))| {
            h.rank = (i + 1) as u32;
            h.id = format!("hyp_{:03}", i + 1);
            h
        })
        .collect();

    BoardResponse {
        snapshot: Snapshot {
            id: String::new(),
            hash: String::new(),
            pack_id: pack.pack_id.clone(),
        },
        kpi_contract: contract.clone(),
        hypotheses,
    }
}

fn factor_labels(graph: &Graph, cand: &operators::Candidate) -> String {
    let labels: Vec<String> = cand
        .source_nodes
        .iter()
        .filter_map(|id| graph.node(id))
        .filter(|n| n.has_tag("controllable"))
        .map(|n| n.label.clone())
        .collect();
    if labels.is_empty() {
        "the factor".to_string()
    } else {
        labels.join(" + ")
    }
}

fn node_label(graph: &Graph, id: &str) -> String {
    graph.node(id).map(|n| n.label.clone()).unwrap_or_else(|| id.to_string())
}

/// Метка управляемого рычага (controllable), если он один; иначе все факторы.
fn lever_label(graph: &Graph, cand: &operators::Candidate) -> String {
    cand.controllable
        .as_ref()
        .map(|id| node_label(graph, id))
        .unwrap_or_else(|| factor_labels(graph, cand))
}

fn title(graph: &Graph, cand: &operators::Candidate) -> String {
    let kpi = node_label(graph, &cand.kpi);
    let factors = factor_labels(graph, cand);
    let lever = lever_label(graph, cand);
    match cand.operator {
        "mechanism_path" => format!("Tune {lever} to improve {kpi}"),
        "substitution" => format!("Substitute {lever} on the path to {kpi}"),
        "gap" => format!("Test {factors} together for {kpi}"),
        "contradiction" => format!("Find the boundary condition affecting {kpi}"),
        "analogy_transfer" => format!("Transfer a proven mechanism toward {kpi}"),
        "uncovered_constraint" => format!("Add {kpi} measurement as a required gate"),
        _ => format!("Hypothesis for {kpi}"),
    }
}

fn summary(graph: &Graph, cand: &operators::Candidate) -> String {
    let kpi = node_label(graph, &cand.kpi);
    let factors = factor_labels(graph, cand);
    let lever = lever_label(graph, cand);
    match cand.operator {
        "mechanism_path" => {
            format!("Adjust {lever} to drive {kpi} through the traced causal path.")
        }
        "substitution" => {
            format!("Use {lever} as an alternative route toward {kpi} with the same effect.")
        }
        "gap" => format!(
            "The combination of {factors} is not covered by the corpus; verify it jointly for {kpi}."
        ),
        "contradiction" => {
            format!("Two claims disagree on {kpi}; determine the condition where the effect flips.")
        }
        "analogy_transfer" => {
            format!("Transfer a mechanism proven elsewhere onto the path to {kpi}.")
        }
        "uncovered_constraint" => {
            format!("The corpus lacks evidence for {kpi}; gate candidates with direct measurement.")
        }
        _ => format!("Hypothesis addressing {kpi}."),
    }
}
