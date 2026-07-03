//! Data Readiness: честная оценка качества исходных xlsx по `data_quality`
//! (ref_error/merged_cell/checksum_mismatch...). Показывает, что и как было
//! обработано детерминированно — зрелость инженерии, а не слабость.

use std::collections::BTreeMap;

use contracts::{DataReadiness, DiagnosticsReport};

pub fn readiness(d: &DiagnosticsReport) -> DataReadiness {
    let mut issues_by_type: BTreeMap<String, usize> = BTreeMap::new();
    for i in &d.data_quality {
        *issues_by_type.entry(i.issue.clone()).or_insert(0) += 1;
    }
    let issues_total = d.data_quality.len();
    let loss_cells = d.loss_cells.len();

    // Доля «здоровых» единиц данных: распарсенные ячейки против проблемных.
    let denom = (loss_cells + issues_total) as f64;
    let readiness_pct = if denom == 0.0 {
        100.0
    } else {
        ((loss_cells as f64 / denom) * 1000.0).round() / 10.0
    };

    let note = format!(
        "{loss_cells} loss cells parsed; {issues_total} data-quality issues handled deterministically",
    );

    DataReadiness {
        factory_id: d.factory_id.clone(),
        readiness_pct,
        loss_cells,
        issues_total,
        issues_by_type,
        note,
    }
}
