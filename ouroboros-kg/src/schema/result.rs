//! Result node CRUD operations

use crate::error::{Neo4jError, Result as CrateResult};
use crate::schema::types::Result;
use chrono::{DateTime, Utc};
use neo4rs::{query, Graph};

/// Create a new Result node in the knowledge graph
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `result` - Result to create
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::{Result as TaskResult, create_result}};
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
///     let result = TaskResult::new(
///         "result-001".to_string(),
///         "Task completed successfully".to_string()
///     );
///
///     create_result(client.graph(), &result).await?;
///     Ok(())
/// }
/// ```
pub async fn create_result(graph: &Graph, result: &Result) -> CrateResult<()> {
    let cypher = query(
        "CREATE (r:Result {
            id: $id,
            content: $content,
            created_at: datetime($created_at)
        })",
    )
    .param("id", result.id.clone())
    .param("content", result.content.clone())
    .param("created_at", result.created_at.to_rfc3339());

    graph
        .run(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to create result: {}", e)))?;

    Ok(())
}

/// Link a Result node to a Task node via PRODUCED relationship
///
/// Creates a relationship: (Task)-[:PRODUCED]->(Result)
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `task_id` - ID of the task that produced the result
/// * `result_id` - ID of the result
///
/// # Returns
/// * `Ok(true)` if both nodes exist and relationship was created
/// * `Ok(false)` if either node doesn't exist
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::link_result_to_task};
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
///     let linked = link_result_to_task(
///         client.graph(),
///         "task-001",
///         "result-001"
///     ).await?;
///
///     println!("Linked: {}", linked);
///     Ok(())
/// }
/// ```
pub async fn link_result_to_task(
    graph: &Graph,
    task_id: &str,
    result_id: &str,
) -> CrateResult<bool> {
    let cypher = query(
        "MATCH (t:Task {id: $task_id})
         MATCH (r:Result {id: $result_id})
         MERGE (t)-[:PRODUCED]->(r)
         RETURN t, r",
    )
    .param("task_id", task_id.to_string())
    .param("result_id", result_id.to_string());

    let mut result = graph
        .execute(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to link result to task: {}", e)))?;

    // Check if any row was returned (both nodes exist and relationship created)
    let linked = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read link result: {}", e)))?
        .is_some();

    Ok(linked)
}

/// Get a Result node by ID from the knowledge graph
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `result_id` - ID of the result to retrieve
///
/// # Returns
/// * `Ok(Some(Result))` if result exists
/// * `Ok(None)` if result not found
/// * `Err(Neo4jError)` on failure
pub async fn get_result(graph: &Graph, result_id: &str) -> CrateResult<Option<Result>> {
    let cypher = query("MATCH (r:Result {id: $id}) RETURN r").param("id", result_id.to_string());

    let mut result = graph
        .execute(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to get result: {}", e)))?;

    if let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read result: {}", e)))?
    {
        let node: neo4rs::Node = row.get("r").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract result node: {}", e))
        })?;

        let id: String = node.get("id").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract result id: {}", e))
        })?;

        let content: String = node.get("content").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract result content: {}", e))
        })?;

        let created_at_str: String = node.get("created_at").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract result created_at: {}", e))
        })?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| {
                Neo4jError::QueryError(format!("Failed to parse created_at datetime: {}", e))
            })?
            .with_timezone(&Utc);

        Ok(Some(Result::with_timestamp(id, content, created_at)))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_creation() {
        let result = Result::new("result-001".to_string(), "Test content".to_string());

        assert_eq!(result.id, "result-001");
        assert_eq!(result.content, "Test content");
    }
}
