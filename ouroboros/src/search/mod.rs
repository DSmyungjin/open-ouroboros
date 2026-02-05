//! Search module for Ouroboros
//!
//! Provides hybrid search capabilities combining:
//! - Vector search (LanceDB) for semantic similarity
//! - Keyword search (Tantivy) for BM25-based matching
//! - Hybrid fusion for combined results
//! - Multilingual support (English/Korean)

pub mod embedding;
pub mod engine;
pub mod hybrid;
pub mod keyword;
pub mod types;
pub mod vector;

pub use embedding::EmbeddingGenerator;
pub use engine::{SearchEngine, SearchMode};
pub use hybrid::{FusionStrategy, HybridConfig, HybridSearch};
pub use keyword::KeywordSearch;
pub use types::{DocumentType, SearchDocument, SearchOptions, SearchResult, SearchSource};
pub use vector::VectorSearch;
