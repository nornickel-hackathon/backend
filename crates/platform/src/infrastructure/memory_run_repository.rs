//! `RunRepository` в памяти процесса (P1.3).

use std::collections::HashMap;
use std::sync::RwLock;

use crate::application::ports::RunRepository;
use crate::application::run_record::RunRecord;

#[derive(Default)]
struct Inner {
    runs: HashMap<String, RunRecord>,
    last: Option<String>,
}

#[derive(Default)]
pub struct MemoryRunRepository {
    inner: RwLock<Inner>,
}

impl RunRepository for MemoryRunRepository {
    fn store(&self, run: RunRecord) {
        let mut inner = self.inner.write().unwrap();
        inner.last = Some(run.run_id.clone());
        inner.runs.insert(run.run_id.clone(), run);
    }

    fn last(&self) -> Option<RunRecord> {
        let inner = self.inner.read().unwrap();
        inner.last.as_ref().and_then(|id| inner.runs.get(id).cloned())
    }
}
