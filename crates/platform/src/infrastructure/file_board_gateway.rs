//! `BoardGateway` поверх `fixtures/board.json` — fallback-портфель до /run.

use std::path::{Path, PathBuf};

use contracts::BoardResponse;

use crate::application::ports::BoardGateway;

pub struct FileBoardGateway {
    path: PathBuf,
}

impl FileBoardGateway {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        FileBoardGateway {
            path: base_dir.as_ref().join("fixtures").join("board.json"),
        }
    }
}

impl BoardGateway for FileBoardGateway {
    fn load(&self) -> Result<BoardResponse, String> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("cannot read board fixture: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("cannot parse board fixture: {e}"))
    }
}
