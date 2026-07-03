//! Generic-операторы генерации кандидатов.
//!
//! Каждый оператор работает только над `EdgeType` и тегами узлов — никаких
//! доменных слов (AGENT_RULES.md §1). Доменная семантика приходит из данных
//! (labels, mechanism) и из pack (`enabled_operators`).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use contracts::{DomainPack, EdgeType, KpiContract, Status};
use petgraph::graph::EdgeIndex;

use crate::graph::Graph;

/// Сырой кандидат гипотезы до scoring/статуса.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub operator: &'static str,
    /// id узлов пути, source-first .. kpi/constraint-last.
    pub source_nodes: Vec<String>,
    pub edges: Vec<EdgeIndex>,
    /// Управляемый фактор (для cost-оценки), если есть.
    pub controllable: Option<String>,
    /// Целевой узел (kpi или constraint).
    pub kpi: String,
    pub trace: Vec<String>,
    pub is_gap: bool,
    pub n_partial_matches: usize,
    pub forced_status: Option<Status>,
}

/// Тип causal-рёбер, по которым строится механизм-путь.
const MECHANISM_EDGES: [EdgeType; 2] = [EdgeType::Mechanism, EdgeType::Proxy];

pub fn generate(graph: &Graph, contract: &KpiContract, pack: &DomainPack) -> Vec<Candidate> {
    let excluded: HashSet<&str> = contract.excluded_factors.iter().map(String::as_str).collect();
    let mut cands = Vec::new();

    // --- mechanism_path: от каждого KPI назад до controllable-фактора ----------
    let mut levers: Vec<Candidate> = Vec::new();
    if pack.operator_enabled("mechanism_path") {
        for kpi_idx in graph.nodes_with_tag("kpi") {
            let kpi_id = graph.weight(kpi_idx).id.clone();
            for path in graph.enumerate_paths(kpi_idx, &MECHANISM_EDGES, "controllable") {
                let source_nodes: Vec<String> =
                    path.nodes.iter().map(|i| graph.weight(*i).id.clone()).collect();
                let controllable = source_nodes.first().cloned();
                if let Some(c) = &controllable {
                    if excluded.contains(c.as_str()) {
                        continue;
                    }
                }
                let trace = graph.claims_on_edges(&path.edges);
                levers.push(Candidate {
                    operator: "mechanism_path",
                    source_nodes,
                    edges: path.edges,
                    controllable,
                    kpi: kpi_id.clone(),
                    trace,
                    is_gap: false,
                    n_partial_matches: 0,
                    forced_status: None,
                });
            }
        }
    }

    // --- substitution: альтернативный фактор через substitution-ребро ----------
    if pack.operator_enabled("substitution") {
        for base in &levers {
            let Some(ctrl_id) = &base.controllable else { continue };
            let Some(ctrl_idx) = graph.index(ctrl_id) else { continue };
            for (sub_idx, sub_edge) in graph.incoming_edges_of_type(ctrl_idx, EdgeType::Substitution) {
                let sub_id = graph.weight(sub_idx).id.clone();
                if excluded.contains(sub_id.as_str()) {
                    continue;
                }
                let mut source_nodes = vec![sub_id.clone()];
                source_nodes.extend(base.source_nodes.iter().cloned());
                let mut edges = vec![sub_edge];
                edges.extend(base.edges.iter().cloned());
                let trace = graph.claims_on_edges(&edges);
                cands.push(Candidate {
                    operator: "substitution",
                    source_nodes,
                    edges,
                    controllable: Some(sub_id),
                    kpi: base.kpi.clone(),
                    trace,
                    is_gap: false,
                    n_partial_matches: 0,
                    forced_status: None,
                });
            }
        }
    }

    // --- gap: несколько факторов поодиночке ведут к KPI, комбинации нет в корпусе
    if pack.operator_enabled("gap") {
        let mut by_kpi: BTreeMap<String, Vec<&Candidate>> = BTreeMap::new();
        for c in &levers {
            by_kpi.entry(c.kpi.clone()).or_default().push(c);
        }
        for (kpi_id, group) in by_kpi {
            let mut factors: Vec<String> =
                group.iter().filter_map(|c| c.controllable.clone()).collect();
            factors.sort();
            factors.dedup();
            if factors.len() < 2 {
                continue;
            }
            let mut trace = BTreeSet::new();
            for c in &group {
                for t in &c.trace {
                    trace.insert(t.clone());
                }
            }
            let mut source_nodes = factors.clone();
            source_nodes.push(kpi_id.clone());
            cands.push(Candidate {
                operator: "gap",
                source_nodes,
                edges: Vec::new(),
                controllable: None,
                kpi: kpi_id,
                trace: trace.into_iter().collect(),
                is_gap: true,
                n_partial_matches: factors.len(),
                forced_status: Some(Status::NeedsExpertReview),
            });
        }
    }

    // --- contradiction (P1, только если включён в pack) ------------------------
    if pack.operator_enabled("contradiction") {
        cands.extend(contradiction(graph));
    }

    // --- analogy_transfer (P1, только если включён в pack) ---------------------
    if pack.operator_enabled("analogy_transfer") {
        cands.extend(analogy_transfer(graph));
    }

    // --- skeptic-rule: непокрытый constraint -> needs_expert_review ------------
    for cn_idx in graph.nodes_with_tag("constraint") {
        if !graph.node_has_evidenced_edge(cn_idx) {
            let cn = graph.weight(cn_idx).id.clone();
            cands.push(Candidate {
                operator: "uncovered_constraint",
                source_nodes: vec![cn.clone()],
                edges: Vec::new(),
                controllable: None,
                kpi: cn,
                trace: Vec::new(),
                is_gap: false,
                n_partial_matches: 0,
                forced_status: Some(Status::NeedsExpertReview),
            });
        }
    }

    cands.extend(levers);
    cands
}

/// Два ребра в один узел с противоположной polarity -> гипотеза граничного условия.
fn contradiction(graph: &Graph) -> Vec<Candidate> {
    use contracts::Polarity;
    use petgraph::visit::EdgeRef;

    let mut out = Vec::new();
    // Группируем входящие рёбра по целевому узлу, ищем pos+neg.
    for node in graph_node_indices(graph) {
        let mut pos: Option<EdgeIndex> = None;
        let mut neg: Option<EdgeIndex> = None;
        for er in graph.raw().edges_directed(node, petgraph::Direction::Incoming) {
            match er.weight().polarity {
                Some(Polarity::Positive) if pos.is_none() => pos = Some(er.id()),
                Some(Polarity::Negative) if neg.is_none() => neg = Some(er.id()),
                _ => {}
            }
        }
        if let (Some(p), Some(n)) = (pos, neg) {
            let edges = vec![p, n];
            let trace = graph.claims_on_edges(&edges);
            let kpi = graph.weight(node).id.clone();
            out.push(Candidate {
                operator: "contradiction",
                source_nodes: vec![kpi.clone()],
                edges,
                controllable: None,
                kpi,
                trace,
                is_gap: false,
                n_partial_matches: 0,
                forced_status: None,
            });
        }
    }
    out
}

/// Узел Y с тегами как у X, но без подтверждённого пути -> перенос механизма.
/// Минимальная generic-реализация (в демо-паке оператор выключен).
fn analogy_transfer(_graph: &Graph) -> Vec<Candidate> {
    // Placeholder generic-оператор P1: включается pack'ом, требует тегов аналогии,
    // которых нет в демо-паке. Возвращает пусто, пока в данных нет analogy-тегов.
    Vec::new()
}

fn graph_node_indices(graph: &Graph) -> Vec<petgraph::graph::NodeIndex> {
    graph.raw().node_indices().collect()
}
