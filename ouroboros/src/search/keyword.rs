//! Keyword search using Tantivy (BM25)

use std::path::Path;

use anyhow::{Context, Result};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, STORED, INDEXED, TEXT};
use tantivy::schema::Value;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};
use tracing::{debug, info};

use super::types::{DocumentType, SearchDocument, SearchOptions, SearchResult, SearchSource};

/// Keyword search engine using Tantivy BM25
pub struct KeywordSearch {
    index: Index,
    reader: IndexReader,
    writer: Option<IndexWriter>,
    #[allow(dead_code)]
    schema: Schema,
    // Field handles
    id_field: Field,
    doc_type_field: Field,
    title_field: Field,
    content_field: Field,
    session_id_field: Field,
    task_id_field: Field,
    created_at_field: Field,
}

impl KeywordSearch {
    /// Create a new keyword search instance (read-write mode)
    pub fn new(index_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_internal(index_path, true)
    }

    /// Create a reader-only keyword search instance (no write lock acquired)
    /// Use this for search-only operations to avoid lock conflicts
    pub fn new_reader_only(index_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_internal(index_path, false)
    }

    /// Internal constructor with configurable write mode
    fn new_internal(index_path: impl AsRef<Path>, with_writer: bool) -> Result<Self> {
        let index_path = index_path.as_ref();
        info!(
            "Initializing Tantivy index at {:?} (write_mode: {})",
            index_path, with_writer
        );

        // Build schema with default tokenizer for title and content
        let mut schema_builder = Schema::builder();

        // Simple fields (no special tokenization needed)
        let id_field = schema_builder.add_text_field("id", STORED);
        let doc_type_field = schema_builder.add_text_field("doc_type", STORED);
        let session_id_field = schema_builder.add_text_field("session_id", STORED);
        let task_id_field = schema_builder.add_text_field("task_id", STORED);

        // Date field for filtering
        let created_at_field = schema_builder.add_date_field("created_at", INDEXED | STORED);

        // Text fields with default tokenizer (English-optimized)
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();

        // Open or create index
        std::fs::create_dir_all(index_path)?;
        let index = Index::open_or_create(
            tantivy::directory::MmapDirectory::open(index_path)?,
            schema.clone(),
        )?;

        // Create writer only if requested (50MB heap)
        let writer = if with_writer {
            Some(index.writer(50_000_000)?)
        } else {
            None
        };

        // Create reader
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            writer,
            schema,
            id_field,
            doc_type_field,
            title_field,
            content_field,
            session_id_field,
            task_id_field,
            created_at_field,
        })
    }

    /// Index a document (requires write mode)
    pub fn index_document(&mut self, doc: &SearchDocument) -> Result<()> {
        if self.writer.is_none() {
            anyhow::bail!("Cannot index: opened in read-only mode");
        }

        // Delete existing document with same ID first (uses writer internally)
        let _ = self.delete_document(&doc.id);

        let tantivy_doc = doc!(
            self.id_field => doc.id.as_str(),
            self.doc_type_field => doc.doc_type.as_str(),
            self.title_field => doc.title.as_str(),
            self.content_field => doc.content.as_str(),
            self.session_id_field => doc.session_id.as_deref().unwrap_or(""),
            self.task_id_field => doc.task_id.as_deref().unwrap_or(""),
            self.created_at_field => tantivy::DateTime::from_timestamp_secs(doc.created_at.timestamp())
        );

        // Now borrow writer mutably - safe since delete_document completed
        self.writer.as_mut().unwrap().add_document(tantivy_doc)?;
        debug!("Indexed document: {}", doc.id);
        Ok(())
    }

    /// Commit pending changes (requires write mode)
    pub fn commit(&mut self) -> Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer.commit()?;
            self.reader.reload()?;
        }
        Ok(())
    }

    /// Search for documents matching the query
    pub fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        // Build query parser for title and content fields
        let query_parser =
            QueryParser::for_index(&self.index, vec![self.title_field, self.content_field]);

        let parsed_query = query_parser
            .parse_query(query)
            .context("Failed to parse query")?;

        // Execute search
        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(options.limit))
            .context("Search failed")?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher
                .doc(doc_address)
                .context("Failed to retrieve document")?;

            let id = get_text_field(&retrieved_doc, self.id_field);
            let doc_type_str = get_text_field(&retrieved_doc, self.doc_type_field);
            let title = get_text_field(&retrieved_doc, self.title_field);
            let content = get_text_field(&retrieved_doc, self.content_field);
            let session_id = get_text_field(&retrieved_doc, self.session_id_field);

            // Parse document type
            let doc_type = match doc_type_str.as_str() {
                "task" => DocumentType::Task,
                "task_result" => DocumentType::TaskResult,
                "context" => DocumentType::Context,
                "plan" => DocumentType::Plan,
                "validation_report" => DocumentType::ValidationReport,
                "knowledge" => DocumentType::Knowledge,
                _ => continue,
            };

            // Apply filters
            if let Some(filter_type) = &options.doc_type {
                if doc_type != *filter_type {
                    continue;
                }
            }
            if let Some(filter_session) = &options.session_id {
                if session_id != *filter_session {
                    continue;
                }
            }

            // Apply date filters
            if options.date_from.is_some() || options.date_to.is_some() {
                // Extract created_at from document
                if let Some(date_value) = retrieved_doc.get_first(self.created_at_field) {
                    if let Some(tantivy_date) = date_value.as_datetime() {
                        let doc_timestamp = tantivy_date.into_timestamp_secs();

                        // Check date_from filter
                        if let Some(from) = &options.date_from {
                            if doc_timestamp < from.timestamp() {
                                continue;
                            }
                        }

                        // Check date_to filter
                        if let Some(to) = &options.date_to {
                            if doc_timestamp > to.timestamp() {
                                continue;
                            }
                        }
                    }
                }
            }

            // Normalize score to 0-1 range (BM25 scores can be unbounded)
            let normalized_score = 1.0 / (1.0 + (-score).exp());

            if let Some(min_score) = options.min_score {
                if normalized_score < min_score {
                    continue;
                }
            }

            results.push(SearchResult {
                id,
                score: normalized_score,
                doc_type,
                title,
                content,
                source: SearchSource::Keyword,
            });
        }

        Ok(results)
    }

    /// Delete a document by ID (requires write mode)
    pub fn delete_document(&mut self, doc_id: &str) -> Result<()> {
        if let Some(ref mut writer) = self.writer {
            let term = tantivy::Term::from_field_text(self.id_field, doc_id);
            writer.delete_term(term);
            debug!("Deleted document: {}", doc_id);
        }
        Ok(())
    }

    /// Get document count
    pub fn count(&self) -> Result<usize> {
        let searcher = self.reader.searcher();
        Ok(searcher.num_docs() as usize)
    }
}

/// Helper to extract text field value
fn get_text_field(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn test_keyword_search_creation() {
        let dir = tempdir().unwrap();
        let search = KeywordSearch::new(dir.path()).unwrap();
        assert_eq!(search.count().unwrap(), 0);
    }

    #[test]
    fn test_index_and_search() {
        let dir = tempdir().unwrap();
        let mut search = KeywordSearch::new(dir.path()).unwrap();

        let doc = SearchDocument {
            id: "test-001".to_string(),
            doc_type: DocumentType::Task,
            title: "Test task".to_string(),
            content: "This is a test document for keyword search".to_string(),
            session_id: None,
            task_id: None,
            created_at: Utc::now(),
            metadata: None,
        };

        search.index_document(&doc).unwrap();
        search.commit().unwrap();

        let results = search
            .search("keyword search", &SearchOptions::new())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test-001");
    }
}
