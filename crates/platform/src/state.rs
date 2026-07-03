//! Composition root / DI-контейнер. Единственное место, где известны
//! конкретные реализации портов: собирает адаптеры `infrastructure` за
//! трейтами `application::ports` и раздаёт их axum-хендлерам как `State`.

use std::path::PathBuf;
use std::sync::Arc;

use crate::application::ports::{
    BoardGateway, DiagnosticsSource, ExpertHypothesesGateway, ExtractSource, FactoryRepository,
    PackRepository, RunRepository,
};
use crate::infrastructure::{
    FileBoardGateway, FileDiagnosticsSource, FileExpertHypothesesGateway, FileExtractSource,
    FileFactoryRepository, FilePackRepository, HttpDiagnosticsSource, HttpExtractSource,
    MemoryRunRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub sidecar_url: Option<String>,
    pub extract_source: Arc<dyn ExtractSource>,
    pub diagnostics_source: Arc<dyn DiagnosticsSource>,
    pub factories: Arc<dyn FactoryRepository>,
    pub packs: Arc<dyn PackRepository>,
    pub runs: Arc<dyn RunRepository>,
    pub board_gateway: Arc<dyn BoardGateway>,
    pub expert_hypotheses: Arc<dyn ExpertHypothesesGateway>,
}

impl AppState {
    /// Файловые адаптеры относительно `base_dir` (корень данных: docs/) +
    /// in-memory run-стор. Если задан env `SIDECAR_URL` — extract/diagnose берутся
    /// у живого сайдкара с файловым fallback (см. `HttpExtractSource`).
    pub fn new(base_dir: PathBuf) -> Self {
        let sidecar = std::env::var("SIDECAR_URL").ok().filter(|s| !s.is_empty());
        let extract_source: Arc<dyn ExtractSource> = match &sidecar {
            Some(url) => Arc::new(HttpExtractSource::new(
                url.clone(),
                FileExtractSource::new(&base_dir),
            )),
            None => Arc::new(FileExtractSource::new(&base_dir)),
        };
        let diagnostics_source: Arc<dyn DiagnosticsSource> = match &sidecar {
            Some(url) => Arc::new(HttpDiagnosticsSource::new(
                url.clone(),
                FileDiagnosticsSource::new(&base_dir),
            )),
            None => Arc::new(FileDiagnosticsSource::new(&base_dir)),
        };
        AppState {
            sidecar_url: sidecar,
            extract_source,
            diagnostics_source,
            factories: Arc::new(FileFactoryRepository::new(&base_dir)),
            packs: Arc::new(FilePackRepository::new(&base_dir)),
            runs: Arc::new(MemoryRunRepository::default()),
            board_gateway: Arc::new(FileBoardGateway::new(&base_dir)),
            expert_hypotheses: Arc::new(FileExpertHypothesesGateway::new(&base_dir)),
        }
    }

    /// Корень данных: env `NORNIKEL_ROOT` или текущая директория.
    pub fn from_env() -> Self {
        let base = std::env::var("NORNIKEL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        AppState::new(base)
    }
}
