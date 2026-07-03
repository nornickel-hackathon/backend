//! crates/contracts — единый источник структур контракта (см. docs/CONTRACTS.md).
//!
//! Только типы (serde). Без логики, без I/O. Эти структуры не переобъявляются
//! в engine/platform/sidecar/web (AGENT_RULES.md §5).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// ExtractResponse — Python Sidecar -> Rust Platform
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResponse {
    pub pack_id: String,
    #[serde(default)]
    pub documents: Vec<Document>,
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub entities: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub path: String,
    #[serde(default)]
    pub source_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Claim
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub text: String,
    pub source_ref: String,
    pub confidence: f64,
    pub evidence_type: EvidenceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    Literature,
    Experiment,
    ExpertNote,
    DataGap,
    Inferred,
}

impl EvidenceType {
    /// Грунтованное доказательство (literature/experiment), не косвенный вывод.
    /// SCORING.md: plausibility считает долю таких рёбер.
    pub fn is_grounded(self) -> bool {
        matches!(self, EvidenceType::Literature | EvidenceType::Experiment)
    }
}

// ---------------------------------------------------------------------------
// Graph (узлы и рёбра)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub properties: Value,
}

impl GraphNode {
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// `estimated_cost_delta_percent` из properties, если есть.
    pub fn cost_delta_percent(&self) -> Option<f64> {
        self.properties
            .get("estimated_cost_delta_percent")
            .and_then(Value::as_f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Factor,
    Mechanism,
    Property,
    Kpi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub src: String,
    pub dst: String,
    pub edge_type: EdgeType,
    #[serde(default)]
    pub mechanism: Option<String>,
    #[serde(default)]
    pub source_claims: Vec<String>,
    #[serde(default)]
    pub polarity: Option<Polarity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Mechanism,
    Proxy,
    Tradeoff,
    Substitution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarity {
    Positive,
    Negative,
    Nonlinear,
}

// ---------------------------------------------------------------------------
// KpiContract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiContract {
    pub target: Target,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub weights_override: HashMap<String, f64>,
    #[serde(default)]
    pub excluded_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub metric: String,
    pub direction: String,
    #[serde(default)]
    pub minimum_delta_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub metric: String,
    pub op: Op,
    pub value: f64,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    #[serde(rename = "<=")]
    LessEqual,
    #[serde(rename = ">=")]
    GreaterEqual,
}

impl Op {
    /// Истина, если `value` нарушает ограничение `op limit`.
    pub fn is_violated_by(self, value: f64, limit: f64) -> bool {
        match self {
            Op::LessEqual => value > limit,
            Op::GreaterEqual => value < limit,
        }
    }
}

// ---------------------------------------------------------------------------
// Scoring / Hypothesis / Board
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub kpi_impact: f64,
    pub evidence: f64,
    pub plausibility: f64,
    pub cost: f64,
    pub risk: f64,
    pub novelty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoePlan {
    pub objective: String,
    pub factors: Vec<String>,
    pub measurements: Vec<String>,
    pub minimum_runs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub status: Status,
    pub rank: u32,
    pub score_total: f64,
    pub score_breakdown: ScoreBreakdown,
    pub trace: Vec<String>,
    pub source_nodes: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub missing_evidence: Vec<String>,
    pub doe_plan: DoePlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Recommended,
    Watch,
    RejectedByConstraints,
    NeedsExpertReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub hash: String,
    pub pack_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardResponse {
    pub snapshot: Snapshot,
    pub kpi_contract: KpiContract,
    pub hypotheses: Vec<Hypothesis>,
}

// ---------------------------------------------------------------------------
// DomainPack — десериализуется из packs/<pack_id>.yaml
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainPack {
    #[serde(rename = "id", alias = "pack_id")]
    pub pack_id: String,
    #[serde(default)]
    pub scoring_weights: HashMap<String, f64>,
    #[serde(default)]
    pub hard_constraints: Vec<HardConstraint>,
    // packs/<id>.yaml использует ключ `operators`; CONTRACTS.md — `enabled_operators`.
    #[serde(default, alias = "operators")]
    pub enabled_operators: Vec<String>,
    // Прочие поля pack (operators, units, node_types, demo_terms, skeptic_rules ...)
    // ядру не нужны и намеренно игнорируются (не deny_unknown_fields).
}

impl DomainPack {
    pub fn operator_enabled(&self, name: &str) -> bool {
        self.enabled_operators.iter().any(|o| o == name)
    }

    /// Вес измерения с учётом override из контракта. Дефолт 0.0, если не задан.
    pub fn weight(&self, dim: &str, overrides: &HashMap<String, f64>) -> f64 {
        overrides
            .get(dim)
            .or_else(|| self.scoring_weights.get(dim))
            .copied()
            .unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardConstraint {
    pub metric: String,
    pub op: Op,
    pub value: f64,
    #[serde(default)]
    pub unit: Option<String>,
}

// ---------------------------------------------------------------------------
// RerunAction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerunAction {
    pub kind: RerunKind,
    #[serde(default)]
    pub payload: RerunPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RerunKind {
    ExcludeFactor,
    ChangeWeight,
    AddConstraint,
    RelaxConstraint,
}

/// Объединённый payload — поля опциональны, используются в зависимости от `kind`.
/// Покрывает обе формы из CONTRACTS.md и API_CONVENTIONS.md.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RerunPayload {
    #[serde(default)]
    pub factor_id: Option<String>,
    #[serde(default, alias = "constraint_kpi")]
    pub metric: Option<String>,
    #[serde(default)]
    pub op: Option<Op>,
    #[serde(default, alias = "dimension")]
    pub dimension: Option<String>,
    #[serde(default)]
    pub value: Option<f64>,
}

// ---------------------------------------------------------------------------
// Формат ошибок (см. CONTRACTS.md / API_CONVENTIONS.md)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub details: Value,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>, details: Value) -> Self {
        ApiError {
            error: ApiErrorBody {
                code: code.to_string(),
                message: message.into(),
                details,
            },
        }
    }
}
