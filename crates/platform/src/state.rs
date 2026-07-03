//! In-memory run store (P1.3) и доступ к файлам репозитория (fixtures, packs).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use contracts::{BoardResponse, DomainPack, ExtractResponse, KpiContract, Snapshot};

/// Один сохранённый прогон: всё нужное для rerun без повторного extraction.
#[derive(Clone)]
pub struct RunState {
    pub run_id: String,
    pub extract: ExtractResponse,
    pub snapshot: Snapshot,
    pub pack: DomainPack,
    pub contract: KpiContract,
    pub board: BoardResponse,
}

#[derive(Default)]
pub struct Inner {
    pub runs: HashMap<String, RunState>,
    pub last: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<Inner>>,
    pub base_dir: PathBuf,
}

impl AppState {
    pub fn new(base_dir: PathBuf) -> Self {
        AppState {
            inner: Arc::new(RwLock::new(Inner::default())),
            base_dir,
        }
    }

    /// Корень репозитория: env `NORNIKEL_ROOT` или текущая директория.
    pub fn from_env() -> Self {
        let base = std::env::var("NORNIKEL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        AppState::new(base)
    }

    pub fn store(&self, run: RunState) {
        let mut inner = self.inner.write().unwrap();
        inner.last = Some(run.run_id.clone());
        inner.runs.insert(run.run_id.clone(), run);
    }

    pub fn last_run(&self) -> Option<RunState> {
        let inner = self.inner.read().unwrap();
        inner.last.as_ref().and_then(|id| inner.runs.get(id).cloned())
    }

    pub fn fixtures_path(&self, name: &str) -> PathBuf {
        self.base_dir.join("fixtures").join(name)
    }
}
