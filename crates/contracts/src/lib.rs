//! crates/contracts — единый источник структур контракта (см. docs/CONTRACTS.md).
//!
//! Только типы (serde). Без логики, без I/O. Эти структуры не переобъявляются
//! в engine/platform/sidecar/web (AGENT_RULES.md §5).

use std::collections::BTreeMap;

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
    /// Страница PDF-источника; обязателен для PDF, `null` для txt/csv.
    #[serde(default)]
    pub source_page: Option<u32>,
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

    /// Класс капзатрат рычага (`properties.capex_class`): 1 — настройка режима,
    /// 2 — замена узла/детали, 3 — новое оборудование.
    pub fn capex_class(&self) -> Option<u8> {
        self.properties
            .get("capex_class")
            .and_then(Value::as_u64)
            .map(|v| v as u8)
    }

    /// Требуемое оборудование рычага (`properties.equipment_required`).
    pub fn equipment_required(&self) -> Option<&str> {
        self.properties
            .get("equipment_required")
            .and_then(Value::as_str)
    }

    /// Доступность рычага на фабрике; проставляется платформой
    /// (`properties.available`). Отсутствие поля трактуется как доступен.
    pub fn is_available(&self) -> bool {
        self.properties
            .get("available")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    /// Тоннаж, адресуемый диагноз-узлом по элементам
    /// (`properties.addressable_tons = { element: tons, ... }`). Проставляется
    /// платформой из `DiagnosticsReport.diagnosis_summary`.
    pub fn addressable_tons(&self) -> BTreeMap<String, f64> {
        self.properties
            .get("addressable_tons")
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Значение `properties.diagnosis` (id диагноза из данных), если есть.
    pub fn diagnosis_id(&self) -> Option<&str> {
        self.properties.get("diagnosis").and_then(Value::as_str)
    }

    /// Тип рычага (`properties.lever_type`) для бенчмарка против экспертов:
    /// `grinding` | `classification` | `flotation` | `reagents` | `new_equipment` | `automation`.
    pub fn lever_type(&self) -> Option<&str> {
        self.properties.get("lever_type").and_then(Value::as_str)
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
// DiagnosticsReport — Python Sidecar -> Rust Platform (ответ POST /diagnose)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    #[serde(default)]
    pub factory_id: String,
    #[serde(default)]
    pub pack_id: String,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub sections: Vec<String>,
    #[serde(default)]
    pub totals: Totals,
    #[serde(default)]
    pub loss_cells: Vec<LossCell>,
    #[serde(default)]
    pub diagnosis_summary: Vec<DiagnosisSummaryItem>,
    #[serde(default)]
    pub data_quality: Vec<DataQualityIssue>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Totals {
    #[serde(default)]
    pub tails_smt: Option<f64>,
    #[serde(default)]
    pub element_28: ElementTotal,
    #[serde(default)]
    pub element_29: ElementTotal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementTotal {
    #[serde(default)]
    pub pct: Option<f64>,
    #[serde(default)]
    pub tons: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LossCell {
    pub section: String,
    pub size_class: String,
    pub mineral_form: String,
    pub element: String,
    pub tons: f64,
    #[serde(default)]
    pub share_of_class_pct: Option<f64>,
    pub recoverable: bool,
    pub diagnosis: String,
    pub cell_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisSummaryItem {
    pub diagnosis: String,
    pub element: String,
    pub tons: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityIssue {
    pub issue: String,
    pub location: String,
    pub handling: String,
    #[serde(default)]
    pub delta_pct: Option<f64>,
}

// ---------------------------------------------------------------------------
// EconomicEffect — вычисляет engine (не LLM)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EconomicEffect {
    #[serde(default)]
    pub addressable_tons: BTreeMap<String, f64>,
    pub recovery_gain_pct_range: [f64; 2],
    pub value_usd_range: [f64; 2],
    #[serde(default)]
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertMatch {
    pub matched: bool,
    pub expert_hypothesis_id: String,
}

// ---------------------------------------------------------------------------
// FactoryConfig — factories/<factory_id>.yaml
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryConfig {
    pub factory_id: String,
    #[serde(default)]
    pub tails_sections: Vec<String>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItem>,
    // Прочие поля (grinding_stages, ...) ядру не нужны — игнорируются.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentItem {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub present: bool,
}

// ---------------------------------------------------------------------------
// ExpertHypothesis — golden/expert_hypotheses.json (эталон для бенчмарка)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertHypothesis {
    pub id: String,
    pub factory_id: String,
    pub text: String,
    /// `grinding` | `classification` | `flotation` | `reagents` | `new_equipment` | `automation`.
    pub lever_type: String,
    pub diagnosis_hint: String,
}

/// Отчёт бенчмарка: сколько эталонных гипотез экспертов «переоткрыл» engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub factory_id: String,
    pub expert_total: usize,
    pub matched: usize,
    pub coverage_pct: f64,
    pub matches: Vec<BenchmarkMatch>,
    pub unmatched_expert_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMatch {
    pub expert_hypothesis_id: String,
    pub expert_text: String,
    pub hypothesis_id: String,
    pub hypothesis_title: String,
    pub lever_type: String,
    pub diagnosis: String,
}

// ---------------------------------------------------------------------------
// DataReadiness — честность о качестве исходных данных (из data_quality)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReadiness {
    pub factory_id: String,
    pub readiness_pct: f64,
    pub loss_cells: usize,
    pub issues_total: usize,
    pub issues_by_type: BTreeMap<String, usize>,
    pub note: String,
}

// ---------------------------------------------------------------------------
// TraceReport — трассировка гипотезы до источников (claims + ячейки xlsx)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceReport {
    pub hypothesis_id: String,
    pub claims: Vec<ClaimRef>,
    pub source_cells: Vec<SourceCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRef {
    pub id: String,
    pub text: String,
    pub source_ref: String,
    #[serde(default)]
    pub source_page: Option<u32>,
    pub evidence_type: EvidenceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCell {
    pub cell_ref: String,
    pub section: String,
    pub size_class: String,
    pub mineral_form: String,
    pub element: String,
    pub tons: f64,
    pub diagnosis: String,
}

// ---------------------------------------------------------------------------
// FactorySummary — сводка по фабрике для мультифабричной карты денег
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorySummary {
    pub factory_id: String,
    pub sections: Vec<String>,
    /// Извлекаемый тоннаж (кроме not_recoverable) по элементам.
    pub recoverable_tons: BTreeMap<String, f64>,
    /// Оценка «денег на столе»: Σ addressable × mid(gain) × price.
    pub opportunity_usd_mid: f64,
    pub n_hypotheses: usize,
    pub n_recommended: usize,
    pub top_hypothesis: Option<String>,
    pub expert_coverage_pct: f64,
}

// ---------------------------------------------------------------------------
// RoadmapPlan — рекомендованный план действий (де-дубль стоимости по диагнозам)
// ---------------------------------------------------------------------------

/// План внедрения: по каждому диагнозу потерь выбирается лучшее (обычно самое
/// дешёвое эффективное) действие; стоимость суммируется по РАЗНЫМ диагнозам
/// (без двойного счёта — гипотезы одного диагноза делят один addressable_tons).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapPlan {
    pub factory_id: String,
    pub max_capex_class: u8,
    /// Честный суммарный эффект: Σ по покрытым диагнозам (не по всем гипотезам).
    pub total_value_usd_range: [f64; 2],
    pub covered_diagnoses: usize,
    /// Диагнозы, не покрытые ни одним действием в рамках бюджета `max_capex_class`.
    pub uncovered_diagnoses: Vec<String>,
    pub phases: Vec<RoadmapPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapPhase {
    pub capex_class: u8,
    pub label: String,
    pub value_usd_range: [f64; 2],
    pub items: Vec<RoadmapItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapItem {
    pub diagnosis: String,
    pub hypothesis_id: String,
    pub title: String,
    pub status: Status,
    pub capex_class: u8,
    pub value_usd_range: [f64; 2],
    pub addressable_tons: BTreeMap<String, f64>,
    /// Прочие гипотезы того же диагноза (id) — альтернативы выбранному действию.
    pub alternatives: Vec<String>,
}

// ---------------------------------------------------------------------------
// KpiContract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiContract {
    #[serde(default)]
    pub factory_id: String,
    pub target: Target,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    /// Цены анонимизированных металлов ($/т); параметр пользователя (дефолты в fixtures).
    #[serde(default)]
    pub prices_usd_per_t: BTreeMap<String, f64>,
    #[serde(default)]
    pub weights_override: BTreeMap<String, f64>,
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
    #[serde(default)]
    pub economic_effect: EconomicEffect,
    pub trace: Vec<String>,
    pub source_nodes: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub missing_evidence: Vec<String>,
    pub doe_plan: DoePlan,
    #[serde(default)]
    pub expert_match: Option<ExpertMatch>,
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
    /// Полный отчёт диагностики — фронту для heatmap и Data Readiness.
    #[serde(default)]
    pub diagnostics: DiagnosticsReport,
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
    pub scoring_weights: BTreeMap<String, f64>,
    #[serde(default)]
    pub hard_constraints: Vec<HardConstraint>,
    // packs/<id>.yaml использует ключ `operators`; CONTRACTS.md — `enabled_operators`.
    #[serde(default, alias = "operators")]
    pub enabled_operators: Vec<String>,
    /// Консервативный диапазон прироста извлечения для economic_effect, когда у
    /// пути нет своего диапазона из claims (SCORING.md). Дефолт `[5.0, 15.0]`.
    #[serde(default = "default_gain")]
    pub default_gain_pct_range: [f64; 2],
    // Прочие поля pack (units, node_types, demo_terms, skeptic_rules ...)
    // ядру не нужны и намеренно игнорируются (не deny_unknown_fields).
}

fn default_gain() -> [f64; 2] {
    [5.0, 15.0]
}

impl DomainPack {
    pub fn operator_enabled(&self, name: &str) -> bool {
        self.enabled_operators.iter().any(|o| o == name)
    }

    /// Вес измерения с учётом override из контракта. Дефолт 0.0, если не задан.
    pub fn weight(&self, dim: &str, overrides: &BTreeMap<String, f64>) -> f64 {
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
    ChangePrice,
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
    // change_price
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub usd_per_t: Option<f64>,
}

// ---------------------------------------------------------------------------
// Constraint parsing — Frontend -> Rust Platform -> Python Sidecar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintParseRequest {
    pub text: String,
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintParseSidecarRequest {
    pub text: String,
    pub kpi_contract: KpiContract,
    pub pack_id: String,
    #[serde(default)]
    pub factors: Vec<ConstraintFactor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintFactor {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstraintParseResponse {
    #[serde(default)]
    pub actions: Vec<RerunAction>,
    #[serde(default)]
    pub kpi_contract_patch: Value,
    #[serde(default)]
    pub unparsed: Vec<String>,
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
