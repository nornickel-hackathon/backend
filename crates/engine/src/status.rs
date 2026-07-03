//! Назначение статуса (код, не LLM). Порядок правил — строго по docs/SCORING.md.

use contracts::{DomainPack, KpiContract, Status};

use crate::graph::Graph;
use crate::operators::Candidate;

/// Свойство-узла с оценкой относительного роста стоимости.
const COST_PROPERTY: &str = "estimated_cost_delta_percent";
const COST_METRIC: &str = "cost";

pub fn assign(
    graph: &Graph,
    cand: &Candidate,
    score_total: f64,
    contract: &KpiContract,
    pack: &DomainPack,
) -> Status {
    if violates_hard_constraint(graph, cand, contract, pack) {
        return Status::RejectedByConstraints;
    }
    if let Some(s) = cand.forced_status {
        return s;
    }
    if score_total >= 0.75 {
        Status::Recommended
    } else {
        // SCORING.md: и >=0.55, и иначе -> watch.
        Status::Watch
    }
}

/// Единственная численная величина гипотезы — cost_delta управляемого фактора;
/// проверяем её против cost-ограничений контракта и пака.
fn violates_hard_constraint(
    graph: &Graph,
    cand: &Candidate,
    contract: &KpiContract,
    pack: &DomainPack,
) -> bool {
    let Some(delta) = cand
        .controllable
        .as_ref()
        .and_then(|id| graph.node(id))
        .and_then(|n| n.properties.get(COST_PROPERTY).and_then(serde_json::Value::as_f64))
    else {
        return false;
    };

    let from_contract = contract
        .constraints
        .iter()
        .filter(|c| normalize(&c.metric) == COST_METRIC)
        .any(|c| c.op.is_violated_by(delta, c.value));

    let from_pack = pack
        .hard_constraints
        .iter()
        .filter(|c| normalize(&c.metric) == COST_METRIC)
        .any(|c| c.op.is_violated_by(delta, c.value));

    from_contract || from_pack
}

fn normalize(s: &str) -> String {
    s.to_lowercase().replace([' ', '_', '-'], "")
}
