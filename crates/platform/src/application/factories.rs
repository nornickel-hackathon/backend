//! Use case `GET /factories`: мультифабричная карта денег. Прогоняет пайплайн
//! для всех фабрик кейса и агрегирует «деньги на столе» + покрытие экспертов.

use contracts::{FactorySummary, Status};

use crate::application::benchmark::load_experts;
use crate::application::error::UseCaseError;
use crate::application::ports::{
    DiagnosticsSource, ExpertHypothesesGateway, ExtractSource, FactoryRepository, PackRepository,
};
use crate::domain::{annotate, benchmark, money, validation};

/// Фабрики кейса (Пример 1–4).
pub const FACTORY_IDS: [&str; 4] = ["kgmk", "nof_vkr", "nof_med", "tof"];

pub fn execute(
    extract_source: &dyn ExtractSource,
    diagnostics_source: &dyn DiagnosticsSource,
    factories: &dyn FactoryRepository,
    packs: &dyn PackRepository,
    experts_gw: &dyn ExpertHypothesesGateway,
) -> Result<Vec<FactorySummary>, UseCaseError> {
    let base_extract = extract_source.load().map_err(UseCaseError::Internal)?;
    validation::validate(&base_extract).map_err(UseCaseError::Validation)?;
    let experts = load_experts(experts_gw);

    let mut out = Vec::new();
    for fid in FACTORY_IDS {
        // Фабрика без данных не валит весь ответ — просто пропускаем.
        let Ok(diag) = diagnostics_source.load(fid, None, &base_extract.pack_id) else {
            continue;
        };
        let Ok(factory) = factories.load(fid) else {
            continue;
        };

        let contract = annotate::default_contract(fid);
        let mut extract = base_extract.clone();
        annotate::annotate(&mut extract, &diag, &factory);

        let Ok(graph) = engine::Graph::build(&extract) else {
            continue;
        };
        let pack = packs
            .load(&extract.pack_id)
            .map_err(UseCaseError::Internal)?;

        let mut board = engine::discover(&graph, &contract, &pack);
        let report =
            benchmark::match_experts(&mut board.hypotheses, &extract.entities, &experts, fid);
        let (recoverable_tons, opportunity_usd_mid) = money::opportunity(
            &diag,
            &contract.prices_usd_per_t,
            pack.default_gain_pct_range,
        );

        out.push(FactorySummary {
            factory_id: fid.to_string(),
            sections: diag.sections.clone(),
            recoverable_tons,
            opportunity_usd_mid,
            n_hypotheses: board.hypotheses.len(),
            n_recommended: board
                .hypotheses
                .iter()
                .filter(|h| h.status == Status::Recommended)
                .count(),
            top_hypothesis: board.hypotheses.first().map(|h| h.title.clone()),
            expert_coverage_pct: report.coverage_pct,
        });
    }
    Ok(out)
}
