//! Vector search using LanceDB

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;
use tracing::{debug, info};

use super::types::{DocumentType, SearchDocument, SearchOptions, SearchResult, SearchSource};

/// Embedding dimension (placeholder - will be replaced with actual model dimension)
const EMBEDDING_DIM: i32 = 384;

/// Vector search engine using LanceDB
pub struct VectorSearch {
    connection: Connection,
    table_name: String,
}

impl VectorSearch {
    /// Create a new vector search instance
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();
        info!("Initializing LanceDB at {:?}", db_path);

        let connection = lancedb::connect(db_path.to_string_lossy().as_ref())
            .execute()
            .await
            .context("Failed to connect to LanceDB")?;

        Ok(Self {
            connection,
            table_name: "documents".to_string(),
        })
    }

    /// Get or create the documents table
    async fn get_or_create_table(&self) -> Result<Table> {
        let table_names = self.connection.table_names().execute().await?;

        if table_names.contains(&self.table_name) {
            debug!("Opening existing table: {}", self.table_name);
            self.connection
                .open_table(&self.table_name)
                .execute()
                .await
                .context("Failed to open table")
        } else {
            info!("Creating new table: {}", self.table_name);
            self.create_table().await
        }
    }

    /// Create the documents table with schema
    async fn create_table(&self) -> Result<Table> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("doc_type", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("session_id", DataType::Utf8, true),
            Field::new("task_id", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("metadata", DataType::Utf8, true),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    EMBEDDING_DIM,
                ),
                false,
            ),
        ]));

        // Create empty batch for table creation
        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        self.connection
            .create_table(&self.table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create table")
    }

    /// Index a document with its embedding
    pub async fn index_document(&self, doc: &SearchDocument, embedding: Vec<f32>) -> Result<()> {
        if embedding.len() != EMBEDDING_DIM as usize {
            anyhow::bail!(
                "Embedding dimension mismatch: expected {}, got {}",
                EMBEDDING_DIM,
                embedding.len()
            );
        }

        let table = self.get_or_create_table().await?;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("doc_type", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("session_id", DataType::Utf8, true),
            Field::new("task_id", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("metadata", DataType::Utf8, true),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    EMBEDDING_DIM,
                ),
                false,
            ),
        ]));

        let id_array = StringArray::from(vec![doc.id.as_str()]);
        let doc_type_array = StringArray::from(vec![doc.doc_type.as_str()]);
        let title_array = StringArray::from(vec![doc.title.as_str()]);
        let content_array = StringArray::from(vec![doc.content.as_str()]);
        let session_id_array = StringArray::from(vec![doc.session_id.as_deref()]);
        let task_id_array = StringArray::from(vec![doc.task_id.as_deref()]);
        let created_at_array = StringArray::from(vec![doc.created_at.to_rfc3339()]);
        let metadata_array = StringArray::from(vec![doc
            .metadata
            .as_ref()
            .map(|m| m.to_string())
            .as_deref()]);

        let embedding_values = Float32Array::from(embedding);
        let field = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array = FixedSizeListArray::try_new(
            field,
            EMBEDDING_DIM,
            Arc::new(embedding_values),
            None,
        )?;

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array),
                Arc::new(doc_type_array),
                Arc::new(title_array),
                Arc::new(content_array),
                Arc::new(session_id_array),
                Arc::new(task_id_array),
                Arc::new(created_at_array),
                Arc::new(metadata_array),
                Arc::new(vector_array),
            ],
        )?;

        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table.add(Box::new(batches)).execute().await?;

        debug!("Indexed document: {}", doc.id);
        Ok(())
    }

    /// Search for similar documents using vector similarity
    pub async fn search(
        &self,
        query_embedding: Vec<f32>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        if query_embedding.len() != EMBEDDING_DIM as usize {
            anyhow::bail!(
                "Query embedding dimension mismatch: expected {}, got {}",
                EMBEDDING_DIM,
                query_embedding.len()
            );
        }

        let table = self.get_or_create_table().await?;

        let mut query = table
            .vector_search(query_embedding)?
            .limit(options.limit);

        // Apply filters if specified
        if let Some(doc_type) = &options.doc_type {
            query = query.only_if(format!("doc_type = '{}'", doc_type.as_str()));
        }
        if let Some(session_id) = &options.session_id {
            query = query.only_if(format!("session_id = '{}'", session_id));
        }

        let results = query.execute().await?;
        let batches: Vec<RecordBatch> = results.try_collect().await?;

        let mut search_results = Vec::new();

        for batch in batches {
            let id_col = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let doc_type_col = batch
                .column_by_name("doc_type")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let title_col = batch
                .column_by_name("title")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let content_col = batch
                .column_by_name("content")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let score_col = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

            if let (Some(ids), Some(doc_types), Some(titles), Some(contents), Some(scores)) =
                (id_col, doc_type_col, title_col, content_col, score_col)
            {
                for i in 0..batch.num_rows() {
                    let distance = scores.value(i);
                    // Convert distance to similarity score (1 / (1 + distance))
                    let score = 1.0 / (1.0 + distance);

                    if let Some(min_score) = options.min_score {
                        if score < min_score {
                            continue;
                        }
                    }

                    let doc_type = match doc_types.value(i) {
                        "task" => DocumentType::Task,
                        "task_result" => DocumentType::TaskResult,
                        "context" => DocumentType::Context,
                        "plan" => DocumentType::Plan,
                        "validation_report" => DocumentType::ValidationReport,
                        "knowledge" => DocumentType::Knowledge,
                        _ => continue,
                    };

                    search_results.push(SearchResult {
                        id: ids.value(i).to_string(),
                        score,
                        doc_type,
                        title: titles.value(i).to_string(),
                        content: contents.value(i).to_string(),
                        source: SearchSource::Vector,
                    });
                }
            }
        }

        // Sort by score descending
        search_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(search_results)
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, doc_id: &str) -> Result<()> {
        let table = self.get_or_create_table().await?;
        table.delete(&format!("id = '{}'", doc_id)).await?;
        debug!("Deleted document: {}", doc_id);
        Ok(())
    }

    /// Get document count
    pub async fn count(&self) -> Result<usize> {
        let table = self.get_or_create_table().await?;
        Ok(table.count_rows(None).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_vector_search_creation() {
        let dir = tempdir().unwrap();
        let search = VectorSearch::new(dir.path()).await.unwrap();
        assert_eq!(search.count().await.unwrap(), 0);
    }
}
