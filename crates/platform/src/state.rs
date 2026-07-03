//! Composition root / DI-контейнер. Единственное место, где известны
//! конкретные реализации портов: собирает адаптеры `infrastructure` за
//! трейтами `application::ports` и раздаёт их axum-хендлерам как `State`.

use std::path::PathBuf;
use std::sync::Arc;

use crate::application::ports::{BoardGateway, ExtractSource, PackRepository, RunRepository};
use crate::infrastructure::{
    FileBoardGateway, FileExtractSource, FilePackRepository, MemoryRunRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub extract_source: Arc<dyn ExtractSource>,
    pub packs: Arc<dyn PackRepository>,
    pub runs: Arc<dyn RunRepository>,
    pub board_gateway: Arc<dyn BoardGateway>,
}

impl AppState {
    /// Файловые адаптеры относительно `base_dir` (корень репозитория) +
    /// in-memory run-стор.
    pub fn new(base_dir: PathBuf) -> Self {
        AppState {
            extract_source: Arc::new(FileExtractSource::new(&base_dir)),
            packs: Arc::new(FilePackRepository::new(&base_dir)),
            runs: Arc::new(MemoryRunRepository::default()),
            board_gateway: Arc::new(FileBoardGateway::new(&base_dir)),
        }
    }

    /// Корень репозитория: env `NORNIKEL_ROOT` или текущая директория.
    pub fn from_env() -> Self {
        let base = std::env::var("NORNIKEL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        AppState::new(base)
    }
}
