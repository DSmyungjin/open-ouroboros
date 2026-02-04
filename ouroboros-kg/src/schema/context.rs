//! Context node CRUD operations

use crate::error::{Neo4jError, Result};
use crate::schema::types::Context;
use neo4rs::{query, Graph};
use serde_json::Value as JsonValue;

/// Create a new Context node in the knowledge graph
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `context` - Context to create
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::{Context, create_context}};
/// use serde_json::json;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let client = Neo4jClient::new(
///         "bolt://localhost:7687",
///         "neo4j",
///         "password",
///         "neo4j"
///     ).await?;
///
///     let context = Context::with_metadata(
///         "ctx-001".to_string(),
///         "API Documentation".to_string(),
///         json!({"version": "1.0", "url": "https://example.com"})
///     );
///
///     create_context(client.graph(), &context).await?;
///     Ok(())
/// }
/// ```
pub async fn create_context(graph: &Graph, context: &Context) -> Result<()> {
    // Serialize metadata to JSON string for storage
    let metadata_str = serde_json::to_string(&context.metadata)
        .map_err(|e| Neo4jError::SerializationError(format!("Failed to serialize metadata: {}", e)))?;

    let cypher = query(
        "CREATE (c:Context {
            id: $id,
            name: $name,
            metadata: $metadata
        })",
    )
    .param("id", context.id.clone())
    .param("name", context.name.clone())
    .param("metadata", metadata_str);

    graph
        .run(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to create context: {}", e)))?;

    Ok(())
}

/// Link a Context node to a Result node via REFERENCES relationship
///
/// Creates a relationship: (Result)-[:REFERENCES]->(Context)
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `result_id` - ID of the result that references the context
/// * `context_id` - ID of the context
///
/// # Returns
/// * `Ok(true)` if both nodes exist and relationship was created
/// * `Ok(false)` if either node doesn't exist
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::link_context_to_result};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let client = Neo4jClient::new(
///         "bolt://localhost:7687",
///         "neo4j",
///         "password",
///         "neo4j"
///     ).await?;
///
///     let linked = link_context_to_result(
///         client.graph(),
///         "result-001",
///         "ctx-001"
///     ).await?;
///
///     println!("Linked: {}", linked);
///     Ok(())
/// }
/// ```
pub async fn link_context_to_result(
    graph: &Graph,
    result_id: &str,
    context_id: &str,
) -> Result<bool> {
    let cypher = query(
        "MATCH (r:Result {id: $result_id})
         MATCH (c:Context {id: $context_id})
         MERGE (r)-[:REFERENCES]->(c)
         RETURN r, c",
    )
    .param("result_id", result_id.to_string())
    .param("context_id", context_id.to_string());

    let mut result = graph
        .execute(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to link context to result: {}", e)))?;

    // Check if any row was returned (both nodes exist and relationship created)
    let linked = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read link result: {}", e)))?
        .is_some();

    Ok(linked)
}

/// Get a Context node by ID from the knowledge graph
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `context_id` - ID of the context to retrieve
///
/// # Returns
/// * `Ok(Some(Context))` if context exists
/// * `Ok(None)` if context not found
/// * `Err(Neo4jError)` on failure
pub async fn get_context(graph: &Graph, context_id: &str) -> Result<Option<Context>> {
    let cypher = query("MATCH (c:Context {id: $id}) RETURN c").param("id", context_id.to_string());

    let mut result = graph
        .execute(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to get context: {}", e)))?;

    if let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read context: {}", e)))?
    {
        let node: neo4rs::Node = row.get("c").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract context node: {}", e))
        })?;

        let id: String = node.get("id").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract context id: {}", e))
        })?;

        let name: String = node.get("name").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract context name: {}", e))
        })?;

        let metadata_str: String = node.get("metadata").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract context metadata: {}", e))
        })?;

        let metadata: JsonValue = serde_json::from_str(&metadata_str).map_err(|e| {
            Neo4jError::SerializationError(format!("Failed to parse metadata JSON: {}", e))
        })?;

        Ok(Some(Context::with_metadata(id, name, metadata)))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_context_creation() {
        let context = Context::new("ctx-001".to_string(), "Test Context".to_string());

        assert_eq!(context.id, "ctx-001");
        assert_eq!(context.name, "Test Context");
        assert!(context.metadata.is_object());
    }

    #[test]
    fn test_context_with_metadata() {
        let metadata = json!({
            "version": "1.0",
            "tags": ["important", "documentation"]
        });

        let context = Context::with_metadata(
            "ctx-001".to_string(),
            "Test Context".to_string(),
            metadata.clone(),
        );

        assert_eq!(context.id, "ctx-001");
        assert_eq!(context.metadata, metadata);
    }
}
