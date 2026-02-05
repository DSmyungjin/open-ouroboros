//! Embedding generation for vector search

use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::info;

/// Embedding model wrapper
pub struct EmbeddingGenerator {
    model: TextEmbedding,
    dimension: usize,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator with multilingual model
    pub fn new() -> Result<Self> {
        info!("Initializing multilingual embedding model");

        let mut options = InitOptions::default();
        options.model_name = EmbeddingModel::MultilingualE5Small;
        options.show_download_progress = true;

        let model = TextEmbedding::try_new(options)
            .context("Failed to initialize embedding model")?;

        Ok(Self {
            model,
            dimension: 384, // MultilingualE5Small dimension
        })
    }

    /// Create with a specific model
    pub fn with_model(model_name: EmbeddingModel) -> Result<Self> {
        info!("Initializing embedding model: {:?}", model_name);

        let dimension = match model_name {
            EmbeddingModel::MultilingualE5Small => 384,
            EmbeddingModel::MultilingualE5Base => 768,
            EmbeddingModel::MultilingualE5Large => 1024,
            EmbeddingModel::AllMiniLML6V2 => 384,
            EmbeddingModel::BGESmallENV15 => 384,
            EmbeddingModel::BGEBaseENV15 => 768,
            _ => 384, // default
        };

        let mut options = InitOptions::default();
        options.model_name = model_name;
        options.show_download_progress = true;

        let model = TextEmbedding::try_new(options)
            .context("Failed to initialize embedding model")?;

        Ok(Self { model, dimension })
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Generate embedding for a single text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self
            .model
            .embed(vec![text], None)
            .context("Failed to generate embedding")?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))
    }

    /// Generate embeddings for multiple texts
    pub fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        self.model
            .embed(texts, None)
            .context("Failed to generate embeddings")
    }

    /// Generate query embedding (with query prefix for e5 models)
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        // E5 models expect "query: " prefix for queries
        let prefixed = format!("query: {}", query);
        self.embed(&prefixed)
    }

    /// Generate document embedding (with passage prefix for e5 models)
    pub fn embed_document(&self, document: &str) -> Result<Vec<f32>> {
        // E5 models expect "passage: " prefix for documents
        let prefixed = format!("passage: {}", document);
        self.embed(&prefixed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model download
    fn test_embedding_generation() {
        let generator = EmbeddingGenerator::new().unwrap();

        // Test English
        let en_embedding = generator.embed("Hello world").unwrap();
        assert_eq!(en_embedding.len(), 384);

        // Test Korean
        let ko_embedding = generator.embed("안녕하세요").unwrap();
        assert_eq!(ko_embedding.len(), 384);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_query_document_embedding() {
        let generator = EmbeddingGenerator::new().unwrap();

        let query_emb = generator.embed_query("search query").unwrap();
        let doc_emb = generator.embed_document("document content").unwrap();

        assert_eq!(query_emb.len(), 384);
        assert_eq!(doc_emb.len(), 384);
    }
}
