//! `PackRepository` поверх `packs/<pack_id>.yaml` (P1.1).

use std::path::{Path, PathBuf};

use contracts::DomainPack;

use crate::application::ports::PackRepository;

pub struct FilePackRepository {
    packs_dir: PathBuf,
}

impl FilePackRepository {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        FilePackRepository {
            packs_dir: base_dir.as_ref().join("packs"),
        }
    }
}

impl PackRepository for FilePackRepository {
    fn load(&self, pack_id: &str) -> Result<DomainPack, String> {
        let path = self.packs_dir.join(format!("{pack_id}.yaml"));
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read pack '{}': {e}", path.display()))?;
        serde_yaml::from_str(&text).map_err(|e| format!("cannot parse pack '{pack_id}': {e}"))
    }
}
