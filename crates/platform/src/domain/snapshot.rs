//! Детерминированный snapshot+hash графа (P1.2) — граница воспроизводимости.
//! Hash НЕ меняется при rerun (граф тот же).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use contracts::{ExtractResponse, Snapshot};

pub fn snapshot_of(extract: &ExtractResponse, factory_id: &str) -> Snapshot {
    // Канонизируем вход: сортируем все элементы по id, чтобы порядок в JSON
    // не влиял на hash.
    let mut nodes: Vec<String> = extract
        .entities
        .iter()
        .map(|n| format!("{}|{:?}|{}", n.id, n.kind, n.tags.join(",")))
        .collect();
    nodes.sort();

    let mut edges: Vec<String> = extract
        .edges
        .iter()
        .map(|e| format!("{}|{}->{}|{:?}", e.id, e.src, e.dst, e.edge_type))
        .collect();
    edges.sort();

    let mut claims: Vec<String> = extract
        .claims
        .iter()
        .map(|c| format!("{}|{}|{:?}", c.id, c.confidence, c.evidence_type))
        .collect();
    claims.sort();

    let mut hasher = DefaultHasher::new();
    factory_id.hash(&mut hasher);
    extract.pack_id.hash(&mut hasher);
    for s in nodes.iter().chain(edges.iter()).chain(claims.iter()) {
        s.hash(&mut hasher);
    }
    let h = hasher.finish();

    Snapshot {
        id: format!("snapshot_{h:016x}"),
        hash: format!("{h:016x}"),
        pack_id: extract.pack_id.clone(),
    }
}
