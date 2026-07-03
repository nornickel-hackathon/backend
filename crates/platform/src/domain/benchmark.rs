//! Бенчмарк против эталонных гипотез экспертов (golden set). Правило матчинга
//! (CONTRACTS.md): совпадает `lever_type` И пересекается диагноз. Жадно: лучшие
//! (высокоранговые) гипотезы первыми «занимают» непокрытых экспертов, чтобы
//! честно оценить покрытие. Заполняет `Hypothesis.expert_match` и строит отчёт.

use std::collections::HashSet;

use contracts::{
    BenchmarkMatch, BenchmarkReport, ExpertHypothesis, ExpertMatch, GraphNode, Hypothesis,
};

pub fn match_experts(
    hyps: &mut [Hypothesis],
    nodes: &[GraphNode],
    experts: &[ExpertHypothesis],
    factory_id: &str,
) -> BenchmarkReport {
    let by_id: std::collections::HashMap<&str, &GraphNode> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let factory_experts: Vec<&ExpertHypothesis> =
        experts.iter().filter(|e| e.factory_id == factory_id).collect();

    let mut claimed: HashSet<&str> = HashSet::new();
    let mut matches = Vec::new();

    for h in hyps.iter_mut() {
        let lever_type = h.source_nodes.iter().find_map(|id| {
            by_id
                .get(id.as_str())
                .filter(|n| n.has_tag("controllable"))
                .and_then(|n| n.lever_type())
        });
        let diagnosis = h
            .source_nodes
            .iter()
            .find_map(|id| by_id.get(id.as_str()).and_then(|n| n.diagnosis_id()));

        let (Some(lt), Some(dg)) = (lever_type, diagnosis) else {
            h.expert_match = None;
            continue;
        };

        let candidates: Vec<&&ExpertHypothesis> = factory_experts
            .iter()
            .filter(|e| e.lever_type == lt && e.diagnosis_hint == dg)
            .collect();
        if candidates.is_empty() {
            h.expert_match = None;
            continue;
        }

        // Предпочитаем ещё не занятого эксперта (жадно, по рангу гипотез).
        let chosen = candidates
            .iter()
            .find(|e| !claimed.contains(e.id.as_str()))
            .or_else(|| candidates.first())
            .unwrap();

        h.expert_match = Some(ExpertMatch {
            matched: true,
            expert_hypothesis_id: chosen.id.clone(),
        });

        if claimed.insert(chosen.id.as_str()) {
            matches.push(BenchmarkMatch {
                expert_hypothesis_id: chosen.id.clone(),
                expert_text: chosen.text.clone(),
                hypothesis_id: h.id.clone(),
                hypothesis_title: h.title.clone(),
                lever_type: lt.to_string(),
                diagnosis: dg.to_string(),
            });
        }
    }

    let expert_total = factory_experts.len();
    let matched = claimed.len();
    let coverage_pct = if expert_total == 0 {
        0.0
    } else {
        ((matched as f64 / expert_total as f64) * 1000.0).round() / 10.0
    };
    let unmatched_expert_ids = factory_experts
        .iter()
        .filter(|e| !claimed.contains(e.id.as_str()))
        .map(|e| e.id.clone())
        .collect();

    BenchmarkReport {
        factory_id: factory_id.to_string(),
        expert_total,
        matched,
        coverage_pct,
        matches,
        unmatched_expert_ids,
    }
}
