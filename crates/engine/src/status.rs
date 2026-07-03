//! Назначение статуса (код, не LLM). Порядок правил — строго по docs/SCORING.md.

use contracts::{DomainPack, KpiContract, Status};

use crate::graph::Graph;
use crate::operators::Candidate;

/// Метрика-измерение капзатрат рычага в контракте/паке.
const CAPEX_METRIC: &str = "capexclass";

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

/// Хард-фильтры: недоступное оборудование (кроме gap — он и предлагает новое
/// оборудование) и `capex_class` рычага против capex-ограничений. Ограничения из
/// контракта (явно заданы пользователем) применяются и к gap; ограничения пака —
/// нет (gap = новое оборудование, capex_class 3 «по определению»).
fn violates_hard_constraint(
    graph: &Graph,
    cand: &Candidate,
    contract: &KpiContract,
    pack: &DomainPack,
) -> bool {
    let Some(node) = cand.controllable.as_ref().and_then(|id| graph.node(id)) else {
        return false;
    };

    // equipment_not_available: недоступный рычаг вне gap отсекается.
    if !cand.is_gap && !node.is_available() {
        return true;
    }

    let Some(capex) = node.capex_class() else {
        return false;
    };
    let capex = capex as f64;

    let from_contract = contract
        .constraints
        .iter()
        .filter(|c| normalize(&c.metric) == CAPEX_METRIC)
        .any(|c| c.op.is_violated_by(capex, c.value));
    if from_contract {
        return true;
    }
    if cand.is_gap {
        return false;
    }
    pack.hard_constraints
        .iter()
        .filter(|c| normalize(&c.metric) == CAPEX_METRIC)
        .any(|c| c.op.is_violated_by(capex, c.value))
}

fn normalize(s: &str) -> String {
    s.to_lowercase().replace([' ', '_', '-'], "")
}
