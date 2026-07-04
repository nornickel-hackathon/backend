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
    // Дедупликация кандидатов по стабильному ключу.
    let mut seen = BTreeSet::new();
    let mut cands: Vec<(String, operators::Candidate)> = Vec::new();
    for cand in operators::generate(graph, contract, pack) {
        let key = format!("{}|{}", cand.operator, cand.source_nodes.join(","));
        if seen.insert(key.clone()) {
            cands.push((key, cand));
        }
    }

    // Проход 1: экономический эффект каждого кандидата + максимум по портфелю.
    let effects: Vec<contracts::EconomicEffect> = cands
        .iter()
        .map(|(_, c)| scoring::economic(graph, c, contract, pack))
        .collect();
    let max_value_mid = effects
        .iter()
        .map(scoring::value_mid)
        .fold(0.0_f64, f64::max);

    // Проход 2: scoring (kpi_impact нормируется на max), статусы, сборка гипотез.
    let mut scored: Vec<(f64, String, Hypothesis)> = Vec::new();
    for ((key, cand), economic_effect) in cands.into_iter().zip(effects) {
        let value_mid = scoring::value_mid(&economic_effect);
        let s = scoring::score(graph, &cand, contract, value_mid, max_value_mid);
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
            economic_effect,
            trace: cand.trace.clone(),
            source_nodes: cand.source_nodes.clone(),
            risks: s.risks,
            missing_evidence: s.missing_evidence,
            doe_plan,
            expert_match: None,
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
        diagnostics: contracts::DiagnosticsReport::default(),
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
        "фактор".to_string()
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

// Формулировки — презентационная обвязка (generic verbs), не доменная семантика:
// {lever}/{kpi} остаются параметрами из данных (core-discipline: доменные слова
// в pack, не здесь). Русский язык выбран потому, что вся платформа — от UI до
// демо и жюри — русскоязычная; см. docs/QA_DEBRIEF.md.
fn title(graph: &Graph, cand: &operators::Candidate) -> String {
    let kpi = node_label(graph, &cand.kpi);
    let lever = lever_label(graph, cand);
    match cand.operator {
        "mechanism_path" => format!("Настроить «{lever}» для снижения «{kpi}»"),
        "substitution" => format!("Заменить «{lever}» на пути к «{kpi}»"),
        "gap" => format!("Внедрить «{lever}» (новая возможность) для «{kpi}»"),
        "contradiction" => format!("Найти граничное условие, влияющее на «{kpi}»"),
        "analogy_transfer" => format!("Перенести проверенный механизм на путь к «{kpi}»"),
        "uncovered_constraint" => format!("Добавить измерение «{kpi}» как обязательный фильтр"),
        _ => format!("Гипотеза по «{kpi}»"),
    }
}

fn summary(graph: &Graph, cand: &operators::Candidate) -> String {
    let kpi = node_label(graph, &cand.kpi);
    let lever = lever_label(graph, cand);
    match cand.operator {
        "mechanism_path" => {
            format!("Изменить «{lever}», чтобы повлиять на «{kpi}» по прослеженной причинной цепочке.")
        }
        "substitution" => {
            format!("Использовать «{lever}» как альтернативный путь к «{kpi}» с тем же эффектом.")
        }
        "gap" => format!(
            "«{lever}» пока недоступен на этой линии; его внедрение открывает путь к «{kpi}»."
        ),
        "contradiction" => {
            format!("Два источника противоречат друг другу по «{kpi}»; нужно определить граничное условие, при котором эффект меняет знак.")
        }
        "analogy_transfer" => {
            format!("Перенести механизм, доказанный в другом месте, на путь к «{kpi}».")
        }
        "uncovered_constraint" => {
            format!("В корпусе нет доказательств по «{kpi}»; кандидаты нужно фильтровать через прямое измерение.")
        }
        _ => format!("Гипотеза, затрагивающая «{kpi}»."),
    }
}
