//! Трассировка гипотезы до источников: claims -> {текст, страница PDF},
//! диагноз -> ячейки исходного xlsx (`cell_ref`). Требование интерпретируемости
//! ТЗ: каждое число «кликается» до первоисточника.

use std::collections::HashMap;

use contracts::{ClaimRef, DiagnosticsReport, ExtractResponse, Hypothesis, SourceCell, TraceReport};

/// Максимум ячеек-источников в ответе (по убыванию тоннажа).
const MAX_CELLS: usize = 15;

pub fn trace(hyp: &Hypothesis, extract: &ExtractResponse, diag: &DiagnosticsReport) -> TraceReport {
    let claim_by_id: HashMap<&str, &contracts::Claim> =
        extract.claims.iter().map(|c| (c.id.as_str(), c)).collect();
    let claims = hyp
        .trace
        .iter()
        .filter_map(|id| claim_by_id.get(id.as_str()))
        .map(|c| ClaimRef {
            id: c.id.clone(),
            text: c.text.clone(),
            source_ref: c.source_ref.clone(),
            source_page: c.source_page,
            evidence_type: c.evidence_type,
        })
        .collect();

    // Диагноз гипотезы — из её source-узлов.
    let node_by_id: HashMap<&str, &contracts::GraphNode> =
        extract.entities.iter().map(|n| (n.id.as_str(), n)).collect();
    let diagnosis = hyp
        .source_nodes
        .iter()
        .find_map(|id| node_by_id.get(id.as_str()).and_then(|n| n.diagnosis_id()));

    let mut source_cells: Vec<SourceCell> = match diagnosis {
        Some(dg) => diag
            .loss_cells
            .iter()
            .filter(|c| c.diagnosis == dg && c.recoverable && c.tons > 0.0)
            .map(|c| SourceCell {
                cell_ref: c.cell_ref.clone(),
                section: c.section.clone(),
                size_class: c.size_class.clone(),
                mineral_form: c.mineral_form.clone(),
                element: c.element.clone(),
                tons: c.tons,
                diagnosis: c.diagnosis.clone(),
            })
            .collect(),
        None => Vec::new(),
    };
    source_cells.sort_by(|a, b| b.tons.partial_cmp(&a.tons).unwrap_or(std::cmp::Ordering::Equal));
    source_cells.truncate(MAX_CELLS);

    TraceReport {
        hypothesis_id: hyp.id.clone(),
        claims,
        source_cells,
    }
}
