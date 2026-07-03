//! `ExtractSource` поверх `fixtures/extract_response.json`.

use std::path::{Path, PathBuf};

use contracts::ExtractResponse;

use crate::application::ports::ExtractSource;

pub struct FileExtractSource {
    path: PathBuf,
}

impl FileExtractSource {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        FileExtractSource {
            path: base_dir.as_ref().join("fixtures").join("extract_response.json"),
        }
    }
}

impl ExtractSource for FileExtractSource {
    fn load(&self) -> Result<ExtractResponse, String> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("cannot read extract fixture: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("cannot parse extract fixture: {e}"))
    }
}
