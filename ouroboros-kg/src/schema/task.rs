//! Task node CRUD operations

use crate::error::{Neo4jError, Result};
use crate::schema::types::{Task, TaskStatus};
use chrono::{DateTime, Utc};
use neo4rs::{query, Graph};

/// Create a new Task node in the knowledge graph
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `task` - Task to create
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::{Task, create_task}};
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
///     let task = Task::new(
///         "task-001".to_string(),
///         "My Task".to_string(),
///         "Task description".to_string()
///     );
///
///     create_task(client.graph(), &task).await?;
///     Ok(())
/// }
/// ```
pub async fn create_task(graph: &Graph, task: &Task) -> Result<()> {
    let cypher = query(
        "CREATE (t:Task {
            id: $id,
            subject: $subject,
            description: $description,
            status: $status,
            created_at: datetime($created_at)
        })"
    )
    .param("id", task.id.clone())
    .param("subject", task.subject.clone())
    .param("description", task.description.clone())
    .param("status", task.status.as_str())
    .param("created_at", task.created_at.to_rfc3339());

    graph
        .run(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to create task: {}", e)))?;

    Ok(())
}

/// Get a Task node by ID from the knowledge graph
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `task_id` - ID of the task to retrieve
///
/// # Returns
/// * `Ok(Some(Task))` if task exists
/// * `Ok(None)` if task not found
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::get_task};
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
///     let task = get_task(client.graph(), "task-001").await?;
///     if let Some(task) = task {
///         println!("Found task: {}", task.subject);
///     }
///     Ok(())
/// }
/// ```
pub async fn get_task(graph: &Graph, task_id: &str) -> Result<Option<Task>> {
    let cypher = query("MATCH (t:Task {id: $id}) RETURN t").param("id", task_id.to_string());

    let mut result = graph
        .execute(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to get task: {}", e)))?;

    if let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read task result: {}", e)))?
    {
        let node: neo4rs::Node = row
            .get("t")
            .map_err(|e| Neo4jError::QueryError(format!("Failed to extract task node: {}", e)))?;

        let id: String = node.get("id").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract task id: {}", e))
        })?;

        let subject: String = node.get("subject").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract task subject: {}", e))
        })?;

        let description: String = node.get("description").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract task description: {}", e))
        })?;

        let status_str: String = node.get("status").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract task status: {}", e))
        })?;

        let status = TaskStatus::from_str(&status_str)
            .ok_or_else(|| Neo4jError::QueryError(format!("Invalid task status: {}", status_str)))?;

        // Parse datetime from Neo4j
        let created_at_str: String = node.get("created_at").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract task created_at: {}", e))
        })?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| {
                Neo4jError::QueryError(format!("Failed to parse created_at datetime: {}", e))
            })?
            .with_timezone(&Utc);

        Ok(Some(Task::with_status(
            id,
            subject,
            description,
            status,
            created_at,
        )))
    } else {
        Ok(None)
    }
}

/// Update a Task node's status in the knowledge graph
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `task_id` - ID of the task to update
/// * `new_status` - New status for the task
///
/// # Returns
/// * `Ok(true)` if task was found and updated
/// * `Ok(false)` if task was not found
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::{update_task, TaskStatus}};
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
///     let updated = update_task(client.graph(), "task-001", TaskStatus::Completed).await?;
///     println!("Task updated: {}", updated);
///     Ok(())
/// }
/// ```
pub async fn update_task(graph: &Graph, task_id: &str, new_status: TaskStatus) -> Result<bool> {
    let cypher = query(
        "MATCH (t:Task {id: $id})
         SET t.status = $status
         RETURN t",
    )
    .param("id", task_id.to_string())
    .param("status", new_status.as_str());

    let mut result = graph
        .execute(cypher)
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to update task: {}", e)))?;

    // Check if any row was returned (task was found and updated)
    let updated = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read update result: {}", e)))?
        .is_some();

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation_query_params() {
        let task = Task::new(
            "task-001".to_string(),
            "Test Task".to_string(),
            "Description".to_string(),
        );

        assert_eq!(task.id, "task-001");
        assert_eq!(task.status, TaskStatus::Pending);
    }
}
