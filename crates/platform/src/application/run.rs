//! Use case `POST /run`: загрузить extract + диагностику + конфиг фабрики,
//! проаннотировать граф (addressable_tons / доступность рычагов), прогнать engine,
//! зафиксировать snapshot и сохранить прогон. Возвращает `{ run_id, board }`.

use contracts::{ApiError, BoardResponse, DataQualityIssue, DiagnosticsReport, KpiContract};
use serde_json::Value;

use crate::application::benchmark::load_experts;
use crate::application::error::UseCaseError;
use crate::application::ports::{
    DiagnosticsSource, ExpertHypothesesGateway, ExtractSource, FactoryRepository, PackRepository,
    RunRepository,
};
use crate::application::run_record::RunRecord;
use crate::domain::{annotate, benchmark, snapshot, validation};

pub struct RunInput {
    pub factory_id: String,
    pub pack_id: Option<String>,
    pub source_file: Option<String>,
    /// Опционален — если пусто, берётся дефолтный контракт по `factory_id`.
    pub kpi_contract: Option<KpiContract>,
}

pub struct RunOutput {
    pub run_id: String,
    pub board: BoardResponse,
}

#[allow(clippy::too_many_arguments)]
pub fn execute(
    extract_source: &dyn ExtractSource,
    diagnostics_source: &dyn DiagnosticsSource,
    factories: &dyn FactoryRepository,
    packs: &dyn PackRepository,
    experts_gw: &dyn ExpertHypothesesGateway,
    runs: &dyn RunRepository,
    input: RunInput,
) -> Result<RunOutput, UseCaseError> {
    let mut extract = extract_source.load().map_err(UseCaseError::Internal)?;
    validation::validate(&extract).map_err(UseCaseError::Validation)?;

    let pack_id = input.pack_id.unwrap_or_else(|| extract.pack_id.clone());
    let mut diagnostics =
        match diagnostics_source.load(&input.factory_id, input.source_file.as_deref(), &pack_id) {
            Ok(report) => report,
            Err(e) if input.source_file.is_none() => {
                claims_only_diagnostics(&input.factory_id, &pack_id, e)
            }
            Err(e) => return Err(UseCaseError::Internal(e)),
        };
    let factory = factories
        .load(&input.factory_id)
        .map_err(UseCaseError::Internal)?;
    if factory.equipment.is_empty() {
        diagnostics.data_quality.push(DataQualityIssue {
            issue: "parse_warning".to_string(),
            location: format!("factories/{}.yaml", input.factory_id),
            handling: "equipment list not provided — hard-filter disabled".to_string(),
            delta_pct: None,
        });
    }

    let mut contract = input
        .kpi_contract
        .unwrap_or_else(|| annotate::default_contract(&input.factory_id));
    if contract.factory_id.is_empty() {
        contract.factory_id = input.factory_id.clone();
    }

    // Проаннотировать граф диагностикой и доступностью оборудования.
    annotate::annotate(&mut extract, &diagnostics, &factory);

    let graph = engine::Graph::build(&extract)
        .map_err(|m| UseCaseError::Validation(ApiError::new("VALIDATION_ERROR", m, Value::Null)))?;

    let pack = packs.load(&pack_id).map_err(UseCaseError::Internal)?;

    let mut board = engine::discover(&graph, &contract, &pack);
    let snap = snapshot::snapshot_of(&extract, &input.factory_id);
    board.snapshot = snap.clone();
    board.diagnostics = diagnostics.clone();

    // Бенчмарк против экспертов: заполнить expert_match у гипотез.
    let experts = load_experts(experts_gw);
    benchmark::match_experts(
        &mut board.hypotheses,
        &extract.entities,
        &experts,
        &contract.factory_id,
    );

    let run_id = runs.next_run_id();
    runs.store(RunRecord {
        run_id: run_id.clone(),
        extract,
        diagnostics,
        snapshot: snap,
        pack,
        contract,
        board: board.clone(),
    });

    Ok(RunOutput { run_id, board })
}

fn claims_only_diagnostics(factory_id: &str, pack_id: &str, reason: String) -> DiagnosticsReport {
    DiagnosticsReport {
        factory_id: factory_id.to_string(),
        pack_id: pack_id.to_string(),
        source_file: String::new(),
        data_quality: vec![DataQualityIssue {
            issue: "parse_warning".to_string(),
            location: "diagnostics".to_string(),
            handling: format!(
                "no quantitative diagnostics — hypotheses from literature graph only ({reason})"
            ),
            delta_pct: None,
        }],
        ..DiagnosticsReport::default()
    }
}
