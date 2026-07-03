//! Один сохранённый прогон: всё нужное для rerun без повторного extraction.

use contracts::{BoardResponse, DomainPack, ExtractResponse, KpiContract, Snapshot};

#[derive(Clone)]
pub struct RunRecord {
    pub run_id: String,
    pub extract: ExtractResponse,
    pub snapshot: Snapshot,
    pub pack: DomainPack,
    pub contract: KpiContract,
    pub board: BoardResponse,
}
