//! Валидация ВСЕГО входного JSON от сайдкара (AGENT_RULES.md: «Rust валидирует
//! весь JSON»). Ошибки → `ApiError`, web-слой маппит их в 422.

use std::collections::HashSet;

use contracts::{ApiError, ExtractResponse};
use serde_json::json;

pub fn validate(extract: &ExtractResponse) -> Result<(), ApiError> {
    let node_ids: HashSet<&str> = extract.entities.iter().map(|n| n.id.as_str()).collect();
    let claim_ids: HashSet<&str> = extract.claims.iter().map(|c| c.id.as_str()).collect();

    for claim in &extract.claims {
        if !(0.0..=1.0).contains(&claim.confidence) {
            return Err(ApiError::new(
                "VALIDATION_ERROR",
                "claim confidence out of range [0,1]",
                json!({ "claim_id": claim.id, "confidence": claim.confidence }),
            ));
        }
    }

    for edge in &extract.edges {
        if !node_ids.contains(edge.src.as_str()) {
            return Err(ApiError::new(
                "VALIDATION_ERROR",
                "edge src references unknown entity",
                json!({ "edge_id": edge.id, "missing_node": edge.src }),
            ));
        }
        if !node_ids.contains(edge.dst.as_str()) {
            return Err(ApiError::new(
                "VALIDATION_ERROR",
                "edge dst references unknown entity",
                json!({ "edge_id": edge.id, "missing_node": edge.dst }),
            ));
        }
        for c in &edge.source_claims {
            if !claim_ids.contains(c.as_str()) {
                return Err(ApiError::new(
                    "VALIDATION_ERROR",
                    "edge source_claims references unknown claim",
                    json!({ "edge_id": edge.id, "missing_claim": c }),
                ));
            }
        }
    }

    Ok(())
}
