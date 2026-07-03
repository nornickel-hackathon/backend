//! `RunRepository` в памяти процесса (P1.3). Выдаёт стабильные `run_0001`-id.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::application::ports::RunRepository;
use crate::application::run_record::RunRecord;

#[derive(Default)]
struct Inner {
    runs: HashMap<String, RunRecord>,
    last: Option<String>,
    counter: u64,
}

#[derive(Default)]
pub struct MemoryRunRepository {
    inner: RwLock<Inner>,
}

impl RunRepository for MemoryRunRepository {
    fn next_run_id(&self) -> String {
        let mut inner = self.inner.write().unwrap();
        inner.counter += 1;
        format!("run_{:04}", inner.counter)
    }

    fn store(&self, run: RunRecord) {
        let mut inner = self.inner.write().unwrap();
        inner.last = Some(run.run_id.clone());
        inner.runs.insert(run.run_id.clone(), run);
    }

    fn get(&self, run_id: &str) -> Option<RunRecord> {
        self.inner.read().unwrap().runs.get(run_id).cloned()
    }

    fn last(&self) -> Option<RunRecord> {
        let inner = self.inner.read().unwrap();
        inner.last.as_ref().and_then(|id| inner.runs.get(id).cloned())
    }
}
