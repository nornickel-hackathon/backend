//! Рекомендованный план действий. Ключевая идея — честная де-дубликация денег:
//! гипотезы одного диагноза делят общий `addressable_tons`, поэтому их value
//! НЕЛЬЗЯ складывать. По каждому диагнозу берём лучшее (по score) действие в
//! рамках бюджета `max_capex`, суммируем стоимость ТОЛЬКО по разным диагнозам и
//! разносим действия по фазам capex (быстрые настройки → узлы → новое оборудование).

use std::collections::BTreeMap;

use contracts::{GraphNode, Hypothesis, RoadmapItem, RoadmapPhase, RoadmapPlan};

fn phase_label(capex_class: u8) -> &'static str {
    match capex_class {
        1 => "Фаза 1 — быстрые настройки режима (capex 1)",
        2 => "Фаза 2 — замена узлов/деталей (capex 2)",
        3 => "Фаза 3 — новое оборудование (capex 3)",
        _ => "Прочие действия",
    }
}

pub fn build(
    hyps: &[Hypothesis],
    nodes: &[GraphNode],
    factory_id: &str,
    max_capex: u8,
) -> RoadmapPlan {
    let by_id: std::collections::HashMap<&str, &GraphNode> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Для каждой гипотезы вытащим (диагноз, capex рычага).
    struct Enriched<'a> {
        hyp: &'a Hypothesis,
        diagnosis: String,
        capex: u8,
    }
    let mut enriched: Vec<Enriched> = Vec::new();
    for h in hyps {
        let diagnosis = h
            .source_nodes
            .iter()
            .find_map(|id| by_id.get(id.as_str()).and_then(|n| n.diagnosis_id()));
        let capex = h.source_nodes.iter().find_map(|id| {
            by_id
                .get(id.as_str())
                .filter(|n| n.has_tag("controllable"))
                .and_then(|n| n.capex_class())
        });
        let (Some(diagnosis), Some(capex)) = (diagnosis, capex) else { continue };
        if h.economic_effect.value_usd_range[1] <= 0.0 {
            continue;
        }
        enriched.push(Enriched {
            hyp: h,
            diagnosis: diagnosis.to_string(),
            capex,
        });
    }

    // Сгруппировать по диагнозу (стабильный порядок — BTreeMap).
    let mut by_diag: BTreeMap<String, Vec<&Enriched>> = BTreeMap::new();
    for e in &enriched {
        by_diag.entry(e.diagnosis.clone()).or_default().push(e);
    }

    let mut items_by_capex: BTreeMap<u8, Vec<RoadmapItem>> = BTreeMap::new();
    let mut total = [0.0_f64, 0.0_f64];
    let mut covered = 0usize;
    let mut uncovered: Vec<String> = Vec::new();

    for (diagnosis, mut group) in by_diag {
        // Кандидаты в рамках бюджета; лучший — по score_total, тай-брейк дешевле.
        let affordable: Vec<&&Enriched> = group.iter().filter(|e| e.capex <= max_capex).collect();
        if affordable.is_empty() {
            uncovered.push(diagnosis);
            continue;
        }
        let primary = affordable
            .iter()
            .max_by(|a, b| {
                a.hyp
                    .score_total
                    .partial_cmp(&b.hyp.score_total)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.capex.cmp(&a.capex)) // при равенстве — дешевле
            })
            .unwrap();

        // Извлекаем нужное из primary в owned-значения, снимая заимствование group.
        let item = RoadmapItem {
            diagnosis: primary.diagnosis.clone(),
            hypothesis_id: primary.hyp.id.clone(),
            title: primary.hyp.title.clone(),
            status: primary.hyp.status,
            capex_class: primary.capex,
            value_usd_range: primary.hyp.economic_effect.value_usd_range,
            addressable_tons: primary.hyp.economic_effect.addressable_tons.clone(),
            alternatives: Vec::new(),
        };
        total[0] += item.value_usd_range[0];
        total[1] += item.value_usd_range[1];
        covered += 1;

        // Альтернативы того же диагноза (все прочие гипотезы), по рангу.
        group.sort_by_key(|e| e.hyp.rank);
        let alternatives: Vec<String> = group
            .iter()
            .filter(|e| e.hyp.id != item.hypothesis_id)
            .map(|e| e.hyp.id.clone())
            .collect();

        let capex = item.capex_class;
        items_by_capex
            .entry(capex)
            .or_default()
            .push(RoadmapItem { alternatives, ..item });
    }

    let phases = items_by_capex
        .into_iter()
        .map(|(capex_class, mut items)| {
            items.sort_by(|a, b| {
                b.value_usd_range[1]
                    .partial_cmp(&a.value_usd_range[1])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let value = items.iter().fold([0.0, 0.0], |acc, i| {
                [acc[0] + i.value_usd_range[0], acc[1] + i.value_usd_range[1]]
            });
            RoadmapPhase {
                capex_class,
                label: phase_label(capex_class).to_string(),
                value_usd_range: value,
                items,
            }
        })
        .collect();

    RoadmapPlan {
        factory_id: factory_id.to_string(),
        max_capex_class: max_capex,
        total_value_usd_range: total,
        covered_diagnoses: covered,
        uncovered_diagnoses: uncovered,
        phases,
    }
}
