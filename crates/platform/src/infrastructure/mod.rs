//! Infrastructure — конкретные адаптеры портов (frameworks & drivers): файловый
//! I/O из репозитория и in-memory run-стор. Реализует трейты `application::ports`.

mod file_board_gateway;
mod file_diagnostics_source;
mod file_expert_hypotheses_gateway;
mod file_extract_source;
mod file_factory_repository;
mod file_pack_repository;
mod http_sidecar;
mod memory_run_repository;

pub use file_board_gateway::FileBoardGateway;
pub use file_diagnostics_source::FileDiagnosticsSource;
pub use file_expert_hypotheses_gateway::FileExpertHypothesesGateway;
pub use file_extract_source::FileExtractSource;
pub use file_factory_repository::FileFactoryRepository;
pub use file_pack_repository::FilePackRepository;
pub use http_sidecar::{HttpDiagnosticsSource, HttpExtractSource};
pub use memory_run_repository::MemoryRunRepository;
