//! Search module for Ouroboros
//!
//! Provides keyword search capabilities using Tantivy BM25.
//! Supports multilingual content (English/Korean).

pub mod engine;
pub mod keyword;
pub mod types;

pub use engine::{SearchEngine, SearchMode};
pub use keyword::KeywordSearch;
pub use types::{DocumentType, SearchDocument, SearchOptions, SearchResult, SearchSource};
