//! `ExpertHypothesesGateway` поверх `golden/expert_hypotheses.json` (pass-through).

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::application::ports::ExpertHypothesesGateway;

pub struct FileExpertHypothesesGateway {
    path: PathBuf,
}

impl FileExpertHypothesesGateway {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        FileExpertHypothesesGateway {
            path: base_dir.as_ref().join("golden").join("expert_hypotheses.json"),
        }
    }
}

impl ExpertHypothesesGateway for FileExpertHypothesesGateway {
    fn load(&self) -> Result<Value, String> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("cannot read expert hypotheses: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("cannot parse expert hypotheses: {e}"))
    }
}
