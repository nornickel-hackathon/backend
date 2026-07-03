//! Generic-операторы генерации кандидатов.
//!
//! Каждый оператор работает только над `EdgeType` и тегами узлов — никаких
//! доменных слов (AGENT_RULES.md §1). Доменная семантика приходит из данных
//! (labels, mechanism) и из pack (`enabled_operators`).

use std::collections::HashSet;

use contracts::{DomainPack, EdgeType, GraphNode, KpiContract, Status};
use petgraph::graph::{EdgeIndex, NodeIndex};

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
    /// Диагноз-узел на пути (несёт `addressable_tons`), если путь через него ведёт.
    pub diagnosis_node: Option<String>,
    pub trace: Vec<String>,
    pub is_gap: bool,
    pub n_partial_matches: usize,
    pub forced_status: Option<Status>,
}

/// Типы causal-рёбер, по которым строится механизм-путь (включая substitution —
/// альтернативный рычаг с тем же эффектом).
const MECHANISM_EDGES: [EdgeType; 3] =
    [EdgeType::Mechanism, EdgeType::Proxy, EdgeType::Substitution];

/// Суммарный адресуемый тоннаж диагноз-узла (по всем элементам).
fn diagnosis_tons(node: &GraphNode) -> f64 {
    node.addressable_tons().values().sum()
}

/// KPI-узлы, соответствующие метрике цели контракта (по id). Если совпадений
/// нет — все KPI-узлы (fallback, generic).
fn target_kpis(graph: &Graph, contract: &KpiContract) -> Vec<NodeIndex> {
    let all = graph.nodes_with_tag("kpi");
    let metric = normalize(&contract.target.metric);
    if metric.is_empty() {
        return all;
    }
    let matched: Vec<NodeIndex> = all
        .iter()
        .copied()
        .filter(|i| normalize(&graph.weight(*i).id).contains(&metric))
        .collect();
    if matched.is_empty() {
        all
    } else {
        matched
    }
}

fn normalize(s: &str) -> String {
    s.to_lowercase().replace([' ', '_', '-'], "")
}

pub fn generate(graph: &Graph, contract: &KpiContract, pack: &DomainPack) -> Vec<Candidate> {
    let excluded: HashSet<&str> = contract.excluded_factors.iter().map(String::as_str).collect();
    let mut cands = Vec::new();
    let mut levers: Vec<Candidate> = Vec::new();

    // Порог gap-оператора: 5% от суммы адресуемого тоннажа всех диагнозов.
    let total_diag: f64 = graph
        .nodes_with_tag("diagnosis")
        .iter()
        .map(|i| diagnosis_tons(graph.weight(*i)))
        .sum();
    let gap_threshold = 0.05 * total_diag;

    // --- mechanism_path: от целевого KPI назад до controllable-рычага ----------
    // Путь засчитывается, только если проходит через диагноз-узел с tons > 0.
    // Доступный рычаг -> mechanism_path; недоступный -> gap (см. ниже).
    if pack.operator_enabled("mechanism_path") {
        for kpi_idx in target_kpis(graph, contract) {
            let kpi_id = graph.weight(kpi_idx).id.clone();
            for path in graph.enumerate_paths(kpi_idx, &MECHANISM_EDGES, "controllable") {
                let source_nodes: Vec<String> =
                    path.nodes.iter().map(|i| graph.weight(*i).id.clone()).collect();
                let Some(controllable) = source_nodes.first().cloned() else { continue };
                if excluded.contains(controllable.as_str()) {
                    continue;
                }
                // диагноз-узел пути с ненулевым тоннажом — обязателен
                let Some(diag_idx) = path
                    .nodes
                    .iter()
                    .find(|i| {
                        let n = graph.weight(**i);
                        n.has_tag("diagnosis") && diagnosis_tons(n) > 0.0
                    })
                    .copied()
                else {
                    continue;
                };
                let diagnosis_node = Some(graph.weight(diag_idx).id.clone());
                let diag_tons = diagnosis_tons(graph.weight(diag_idx));
                let available = graph.node(&controllable).map(|n| n.is_available()).unwrap_or(true);
                let trace = graph.claims_on_edges(&path.edges);

                if available {
                    levers.push(Candidate {
                        operator: "mechanism_path",
                        source_nodes,
                        edges: path.edges,
                        controllable: Some(controllable),
                        kpi: kpi_id.clone(),
                        diagnosis_node,
                        trace,
                        is_gap: false,
                        n_partial_matches: 0,
                        forced_status: None,
                    });
                } else if pack.operator_enabled("gap") && diag_tons > gap_threshold {
                    // Недоступный рычаг к «крупному» диагнозу -> gap-гипотеза
                    // (нужно новое оборудование/метод). Скоринг решает статус.
                    cands.push(Candidate {
                        operator: "gap",
                        source_nodes,
                        edges: path.edges,
                        controllable: Some(controllable),
                        kpi: kpi_id.clone(),
                        diagnosis_node,
                        trace,
                        is_gap: true,
                        n_partial_matches: 1,
                        forced_status: None,
                    });
                }
            }
        }
    }

    // --- substitution: альтернативный фактор через substitution-ребро в рычаг ---
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
                    diagnosis_node: base.diagnosis_node.clone(),
                    trace,
                    is_gap: false,
                    n_partial_matches: 0,
                    forced_status: None,
                });
            }
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
                diagnosis_node: None,
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
                diagnosis_node: None,
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
