//! Оценка «денег на столе» по фабрике: извлекаемый металл (recoverable
//! loss_cells) × средний прирост извлечения × цена. Для мультифабричной карты.

use std::collections::BTreeMap;

use contracts::DiagnosticsReport;

/// Возвращает (извлекаемый тоннаж по элементам, оценка в USD при mid(gain)).
pub fn opportunity(
    diag: &DiagnosticsReport,
    prices: &BTreeMap<String, f64>,
    gain: [f64; 2],
) -> (BTreeMap<String, f64>, f64) {
    let mid = (gain[0] + gain[1]) / 2.0;
    let mut tons: BTreeMap<String, f64> = BTreeMap::new();
    let mut usd = 0.0;
    for c in &diag.loss_cells {
        if !c.recoverable {
            continue;
        }
        *tons.entry(c.element.clone()).or_insert(0.0) += c.tons;
        let price = prices.get(&c.element).copied().unwrap_or(0.0);
        usd += c.tons * mid / 100.0 * price;
    }
    // Округлим тоннаж до 0.1 для стабильного вывода.
    for v in tons.values_mut() {
        *v = (*v * 10.0).round() / 10.0;
    }
    (tons, (usd).round())
}
