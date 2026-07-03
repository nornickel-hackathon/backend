//! `DiagnosticsSource` поверх `fixtures/diagnostics_<factory_id>.json`.

use std::path::{Path, PathBuf};

use contracts::DiagnosticsReport;

use crate::application::ports::DiagnosticsSource;

pub struct FileDiagnosticsSource {
    fixtures_dir: PathBuf,
}

impl FileDiagnosticsSource {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        FileDiagnosticsSource {
            fixtures_dir: base_dir.as_ref().join("fixtures"),
        }
    }
}

impl DiagnosticsSource for FileDiagnosticsSource {
    fn load(
        &self,
        factory_id: &str,
        source_file: Option<&str>,
        _pack_id: &str,
    ) -> Result<DiagnosticsReport, String> {
        if source_file.is_some() {
            return Err(
                "source_file diagnostics require a live sidecar (set SIDECAR_URL)".to_string(),
            );
        }
        let path = self
            .fixtures_dir
            .join(format!("diagnostics_{factory_id}.json"));
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read diagnostics '{}': {e}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|e| format!("cannot parse diagnostics '{factory_id}': {e}"))
    }
}
