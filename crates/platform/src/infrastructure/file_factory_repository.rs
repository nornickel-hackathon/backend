//! `FactoryRepository` поверх `factories/<factory_id>.yaml`.

use std::path::{Path, PathBuf};

use contracts::FactoryConfig;

use crate::application::ports::FactoryRepository;

pub struct FileFactoryRepository {
    factories_dir: PathBuf,
}

impl FileFactoryRepository {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        FileFactoryRepository {
            factories_dir: base_dir.as_ref().join("factories"),
        }
    }
}

impl FactoryRepository for FileFactoryRepository {
    fn load(&self, factory_id: &str) -> Result<FactoryConfig, String> {
        let path = self.factories_dir.join(format!("{factory_id}.yaml"));
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(FactoryConfig {
                    factory_id: factory_id.to_string(),
                    tails_sections: Vec::new(),
                    equipment: Vec::new(),
                });
            }
            Err(e) => return Err(format!("cannot read factory '{}': {e}", path.display())),
        };
        serde_yaml::from_str(&text).map_err(|e| format!("cannot parse factory '{factory_id}': {e}"))
    }
}
