pub mod api;
pub mod cli;
pub mod dag;
pub mod docs;
pub mod orchestrator;
pub mod search;
pub mod session;
pub mod work_session;

pub use api::{ApiServer, AuthState, JwtAuth};
pub use orchestrator::Orchestrator;
pub use search::{
    DocumentType, EmbeddingGenerator, FusionStrategy, HybridConfig, HybridSearch, KeywordSearch,
    SearchDocument, SearchEngine, SearchMode, SearchOptions, SearchResult, VectorSearch,
};
pub use session::{SessionManager, SessionEntry, SessionsIndex};
pub use work_session::{WorkSession, WorkSessionManager, WorkSessionStatus, WorkSessionsIndex};
