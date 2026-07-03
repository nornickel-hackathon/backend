//! Базовый Experiment Agent: generic DOE-план из labels узлов пути.
//! Никаких доменных строк в коде — всё берётся из `node.label`.

use contracts::DoePlan;

use crate::graph::Graph;
use crate::operators::Candidate;

pub fn plan(graph: &Graph, cand: &Candidate) -> DoePlan {
    let kpi_label = label(graph, &cand.kpi);

    let factors: Vec<String> = cand
        .source_nodes
        .iter()
        .filter_map(|id| graph.node(id))
        .filter(|n| n.has_tag("controllable"))
        .map(|n| n.label.clone())
        .collect();

    let mut measurements: Vec<String> = vec![kpi_label.clone()];
    for id in &cand.source_nodes {
        if let Some(n) = graph.node(id) {
            if n.has_tag("kpi_proxy") {
                measurements.push(n.label.clone());
            }
        }
    }
    measurements.push("cost delta".to_string());
    measurements.dedup();

    let n_factors = factors.len().max(1);
    DoePlan {
        objective: format!("Validate the effect on {kpi_label} for the proposed change."),
        factors: if factors.is_empty() {
            vec![kpi_label]
        } else {
            factors
        },
        measurements,
        minimum_runs: (3 * n_factors) as u32,
    }
}

fn label(graph: &Graph, id: &str) -> String {
    graph.node(id).map(|n| n.label.clone()).unwrap_or_else(|| id.to_string())
}
