//! Загрузка domain pack из packs/<pack_id>.yaml (P1.1).

use std::path::Path;

use contracts::DomainPack;

pub fn load(base_dir: &Path, pack_id: &str) -> Result<DomainPack, String> {
    let path = base_dir.join("packs").join(format!("{pack_id}.yaml"));
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read pack '{}': {e}", path.display()))?;
    serde_yaml::from_str(&text).map_err(|e| format!("cannot parse pack '{pack_id}': {e}"))
}
