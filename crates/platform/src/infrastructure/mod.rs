//! Infrastructure — конкретные адаптеры портов (frameworks & drivers): файловый
//! I/O из репозитория и in-memory run-стор. Реализует трейты `application::ports`.

mod file_board_gateway;
mod file_extract_source;
mod file_pack_repository;
mod memory_run_repository;

pub use file_board_gateway::FileBoardGateway;
pub use file_extract_source::FileExtractSource;
pub use file_pack_repository::FilePackRepository;
pub use memory_run_repository::MemoryRunRepository;
