//! Один сохранённый прогон: всё нужное для rerun без повторного extraction.

use contracts::{
    BoardResponse, DiagnosticsReport, DomainPack, ExtractResponse, KpiContract, Snapshot,
};

#[derive(Clone)]
pub struct RunRecord {
    pub run_id: String,
    /// Extract с уже проаннотированными диагноз-узлами/доступностью рычагов —
    /// rerun переиспользует его без повторной extraction.
    pub extract: ExtractResponse,
    pub diagnostics: DiagnosticsReport,
    pub snapshot: Snapshot,
    pub pack: DomainPack,
    pub contract: KpiContract,
    pub board: BoardResponse,
}
