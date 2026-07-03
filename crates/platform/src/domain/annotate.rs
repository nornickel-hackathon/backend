//! Аннотация графа перед engine (P2.2): диагноз-узлы получают `addressable_tons`
//! из `DiagnosticsReport`, controllable-рычаги — флаг `available` из
//! `FactoryConfig`. Плюс дефолтный `KpiContract` по фабрике. Чистая доменная
//! логика платформы (без I/O, без HTTP).

use std::collections::{BTreeMap, HashMap, HashSet};

use contracts::{
    Constraint, DiagnosticsReport, ExtractResponse, FactoryConfig, KpiContract, Op, Target,
};
use serde_json::{json, Value};

/// Проставить в `extract.entities`:
/// - диагноз-узлам (`tags: ["diagnosis"]`) — `properties.addressable_tons`
///   `{ element: tons }` по `diagnosis_summary`;
/// - controllable-рычагам — `properties.available` = наличие требуемого
///   оборудования на фабрике (`present: true`).
pub fn annotate(
    extract: &mut ExtractResponse,
    diagnostics: &DiagnosticsReport,
    factory: &FactoryConfig,
) {
    // diagnosis -> { element: tons }
    let mut by_diag: HashMap<&str, HashMap<String, f64>> = HashMap::new();
    for item in &diagnostics.diagnosis_summary {
        *by_diag
            .entry(item.diagnosis.as_str())
            .or_default()
            .entry(item.element.clone())
            .or_insert(0.0) += item.tons;
    }

    let equipment_filter_disabled = factory.equipment.is_empty();
    let present: HashSet<&str> = factory
        .equipment
        .iter()
        .filter(|e| e.present)
        .map(|e| e.id.as_str())
        .collect();

    for node in &mut extract.entities {
        if node.has_tag("diagnosis") {
            if let Some(diag) = node.diagnosis_id().map(str::to_string) {
                if let Some(tons) = by_diag.get(diag.as_str()) {
                    set_prop(&mut node.properties, "addressable_tons", json!(tons));
                }
            }
        }
        if node.has_tag("controllable") {
            let available = equipment_filter_disabled
                || match node.equipment_required() {
                    Some(req) => present.contains(req),
                    None => true,
                };
            set_prop(&mut node.properties, "available", json!(available));
        }
    }
}

fn set_prop(properties: &mut Value, key: &str, value: Value) {
    if !properties.is_object() {
        *properties = json!({});
    }
    if let Some(obj) = properties.as_object_mut() {
        obj.insert(key.to_string(), value);
    }
}

/// Дефолтный контракт по фабрике: цель — снизить извлекаемые потери element_28,
/// capex_class <= 3, дефолтные цены (как в fixtures). Используется, когда
/// `POST /run` пришёл без `kpi_contract`.
pub fn default_contract(factory_id: &str) -> KpiContract {
    KpiContract {
        factory_id: factory_id.to_string(),
        target: Target {
            metric: "recoverable_losses_element_28".to_string(),
            direction: "decrease".to_string(),
            minimum_delta_percent: Some(10.0),
        },
        constraints: vec![Constraint {
            metric: "capex_class".to_string(),
            op: Op::LessEqual,
            value: 3.0,
            unit: Some("class".to_string()),
        }],
        prices_usd_per_t: BTreeMap::from([
            ("element_28".to_string(), 16500.0),
            ("element_29".to_string(), 9500.0),
        ]),
        weights_override: BTreeMap::new(),
        excluded_factors: Vec::new(),
    }
}
