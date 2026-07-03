//! Плоская выгрузка портфеля (P0.8): `GET /export/board.csv`. JSON-выгрузка —
//! это сам `BoardResponse`, отдельная сборка не нужна.

use contracts::{BoardResponse, Hypothesis};

/// CSV-представление портфеля. Колонки (ТЗ):
/// rank,id,title,status,score_total,value_usd_lo,value_usd_hi,capex_class,
/// addressable_tons_28,trace (trace через `;`).
pub fn board_csv(board: &BoardResponse) -> String {
    let mut out = String::from(
        "rank,id,title,status,score_total,value_usd_lo,value_usd_hi,capex_class,addressable_tons_28,trace\n",
    );
    for h in &board.hypotheses {
        let value = &h.economic_effect.value_usd_range;
        let addressable_28 = h
            .economic_effect
            .addressable_tons
            .get("element_28")
            .copied()
            .unwrap_or(0.0);
        let row = [
            h.rank.to_string(),
            h.id.clone(),
            h.title.clone(),
            status_str(h),
            format!("{:.3}", h.score_total),
            format!("{:.0}", value[0]),
            format!("{:.0}", value[1]),
            capex_class(h),
            format!("{addressable_28:.1}"),
            h.trace.join(";"),
        ]
        .iter()
        .map(|c| csv_field(c))
        .collect::<Vec<_>>()
        .join(",");
        out.push_str(&row);
        out.push('\n');
    }
    out
}

fn status_str(h: &Hypothesis) -> String {
    serde_json::to_value(h.status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// capex_class рычага восстанавливаем из cost-оси (1.0→1, 0.7→2, 0.35→3).
fn capex_class(h: &Hypothesis) -> String {
    let c = h.score_breakdown.cost;
    if (c - 1.0).abs() < 1e-6 {
        "1".to_string()
    } else if (c - 0.35).abs() < 1e-6 {
        "3".to_string()
    } else if (c - 0.7).abs() < 1e-6 {
        "2".to_string()
    } else {
        String::new()
    }
}

/// Минимальное CSV-экранирование (RFC 4180): кавычки/запятые/переводы строк.
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
