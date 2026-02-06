pub mod api;
pub mod cli;
pub mod dag;
pub mod docs;
pub mod knowledge;
pub mod orchestrator;
pub mod search;
pub mod session;
pub mod work_session;

pub use api::{ApiServer, AuthState, JwtAuth};
pub use knowledge::{KnowledgeCategory, KnowledgeEntry, KnowledgeExtractor};
pub use orchestrator::Orchestrator;
pub use search::{
    DocumentType, KeywordSearch, SearchDocument, SearchEngine, SearchMode, SearchOptions,
    SearchResult, SearchSource,
};
pub use session::{SessionManager, SessionEntry, SessionsIndex};
pub use work_session::{WorkSession, WorkSessionManager, WorkSessionStatus, WorkSessionsIndex};

/// Returns a "Hello, World!" greeting string.
///
/// # Examples
///
/// ```
/// use ouroboros::hello_world;
///
/// let greeting = hello_world();
/// assert_eq!(greeting, "Hello, World!");
/// ```
pub fn hello_world() -> String {
    String::from("Hello, World!")
}

/// Prints "Hello, World!" to stdout.
///
/// # Examples
///
/// ```
/// use ouroboros::print_hello_world;
///
/// print_hello_world();
/// // Outputs: Hello, World!
/// ```
pub fn print_hello_world() {
    println!("Hello, World!");
}
