//! Knowledge node operations for the knowledge graph
//!
//! Knowledge nodes represent accumulated project knowledge including:
//! - Decisions: Technology choices, architectural decisions
//! - Failures: Implementation failures with causes and lessons
//! - Experiments: Hypothesis tests and their results
//! - Learnings: Insights and patterns discovered
//! - Abandoned: Approaches that were tried and discarded

use crate::error::{Neo4jError, Result};
use crate::schema::types::{Knowledge, KnowledgeType};
use neo4rs::{query, Graph};

/// Create a new Knowledge node in the graph
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `knowledge` - Knowledge struct to create
///
/// # Returns
/// * `Ok(String)` - ID of the created knowledge node
/// * `Err(Neo4jError)` on failure
pub async fn create_knowledge(graph: &Graph, knowledge: &Knowledge) -> Result<String> {
    let metadata_str = serde_json::to_string(&knowledge.metadata)
        .map_err(|e| Neo4jError::QueryError(format!("Failed to serialize metadata: {}", e)))?;

    let cypher = query(
        "CREATE (k:Knowledge {
            id: $id,
            knowledge_type: $knowledge_type,
            title: $title,
            content: $content,
            metadata: $metadata,
            created_at: datetime($created_at),
            source_task_id: $source_task_id
        })
        RETURN k.id as id",
    )
    .param("id", knowledge.id.clone())
    .param("knowledge_type", knowledge.knowledge_type.as_str().to_string())
    .param("title", knowledge.title.clone())
    .param("content", knowledge.content.clone())
    .param("metadata", metadata_str)
    .param("created_at", knowledge.created_at.to_rfc3339())
    .param("source_task_id", knowledge.source_task_id.clone().unwrap_or_default());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to create knowledge: {}", e))
    })?;

    if let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read result: {}", e)))?
    {
        let id: String = row.get("id").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract id: {}", e))
        })?;
        Ok(id)
    } else {
        Err(Neo4jError::QueryError("No result returned".to_string()))
    }
}

/// Get a Knowledge node by ID
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `knowledge_id` - ID of the knowledge to retrieve
///
/// # Returns
/// * `Ok(Some(Knowledge))` if found
/// * `Ok(None)` if not found
/// * `Err(Neo4jError)` on failure
pub async fn get_knowledge(graph: &Graph, knowledge_id: &str) -> Result<Option<Knowledge>> {
    let cypher = query(
        "MATCH (k:Knowledge {id: $id})
         RETURN k.id as id, k.knowledge_type as knowledge_type, k.title as title,
                k.content as content, k.metadata as metadata,
                k.created_at as created_at, k.source_task_id as source_task_id",
    )
    .param("id", knowledge_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to get knowledge: {}", e))
    })?;

    if let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read result: {}", e)))?
    {
        let id: String = row.get("id").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract id: {}", e))
        })?;
        let knowledge_type_str: String = row.get("knowledge_type").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract knowledge_type: {}", e))
        })?;
        let title: String = row.get("title").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract title: {}", e))
        })?;
        let content: String = row.get("content").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract content: {}", e))
        })?;
        let metadata_str: String = row.get("metadata").unwrap_or_default();
        let source_task_id: String = row.get("source_task_id").unwrap_or_default();

        let knowledge_type = KnowledgeType::from_str(&knowledge_type_str)
            .ok_or_else(|| Neo4jError::QueryError(format!("Invalid knowledge type: {}", knowledge_type_str)))?;

        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        Ok(Some(Knowledge {
            id,
            knowledge_type,
            title,
            content,
            metadata,
            created_at: chrono::Utc::now(), // TODO: parse from Neo4j datetime
            source_task_id: if source_task_id.is_empty() { None } else { Some(source_task_id) },
        }))
    } else {
        Ok(None)
    }
}

/// Link a Knowledge node to its source Task
///
/// Creates: (Task)-[:PRODUCED_KNOWLEDGE]->(Knowledge)
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `task_id` - ID of the source task
/// * `knowledge_id` - ID of the knowledge node
pub async fn link_knowledge_to_task(
    graph: &Graph,
    task_id: &str,
    knowledge_id: &str,
) -> Result<bool> {
    let cypher = query(
        "MATCH (t:Task {id: $task_id})
         MATCH (k:Knowledge {id: $knowledge_id})
         MERGE (t)-[:PRODUCED_KNOWLEDGE]->(k)
         RETURN t, k",
    )
    .param("task_id", task_id.to_string())
    .param("knowledge_id", knowledge_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to link knowledge to task: {}", e))
    })?;

    let linked = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read result: {}", e)))?
        .is_some();

    Ok(linked)
}

/// Get all knowledge nodes of a specific type
pub async fn get_knowledge_by_type(
    graph: &Graph,
    knowledge_type: KnowledgeType,
) -> Result<Vec<Knowledge>> {
    let cypher = query(
        "MATCH (k:Knowledge {knowledge_type: $knowledge_type})
         RETURN k.id as id, k.knowledge_type as knowledge_type, k.title as title,
                k.content as content, k.metadata as metadata,
                k.source_task_id as source_task_id
         ORDER BY k.created_at DESC",
    )
    .param("knowledge_type", knowledge_type.as_str().to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to query knowledge: {}", e))
    })?;

    let mut knowledge_list = Vec::new();

    while let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read row: {}", e)))?
    {
        let id: String = row.get("id").unwrap_or_default();
        let title: String = row.get("title").unwrap_or_default();
        let content: String = row.get("content").unwrap_or_default();
        let metadata_str: String = row.get("metadata").unwrap_or_default();
        let source_task_id: String = row.get("source_task_id").unwrap_or_default();

        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        knowledge_list.push(Knowledge {
            id,
            knowledge_type,
            title,
            content,
            metadata,
            created_at: chrono::Utc::now(),
            source_task_id: if source_task_id.is_empty() { None } else { Some(source_task_id) },
        });
    }

    Ok(knowledge_list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exists() {
        assert!(true);
    }
}
