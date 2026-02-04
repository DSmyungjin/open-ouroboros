//! Relationship operations for the knowledge graph

use crate::error::{Neo4jError, Result};
use neo4rs::{query, Graph};

/// Create a DEPENDS_ON relationship between two tasks
///
/// Creates a relationship: (dependent_task)-[:DEPENDS_ON]->(prerequisite_task)
/// This indicates that `dependent_task` depends on `prerequisite_task` being completed first.
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `dependent_task_id` - ID of the task that has a dependency
/// * `prerequisite_task_id` - ID of the task that must be completed first
///
/// # Returns
/// * `Ok(true)` if both tasks exist and relationship was created
/// * `Ok(false)` if either task doesn't exist
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::create_dependency};
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
///     // task-002 depends on task-001
///     let created = create_dependency(
///         client.graph(),
///         "task-002",
///         "task-001"
///     ).await?;
///
///     println!("Dependency created: {}", created);
///     Ok(())
/// }
/// ```
pub async fn create_dependency(
    graph: &Graph,
    dependent_task_id: &str,
    prerequisite_task_id: &str,
) -> Result<bool> {
    let cypher = query(
        "MATCH (dependent:Task {id: $dependent_id})
         MATCH (prerequisite:Task {id: $prerequisite_id})
         MERGE (dependent)-[:DEPENDS_ON]->(prerequisite)
         RETURN dependent, prerequisite",
    )
    .param("dependent_id", dependent_task_id.to_string())
    .param("prerequisite_id", prerequisite_task_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to create dependency: {}", e))
    })?;

    // Check if any row was returned (both tasks exist and relationship created)
    let created = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read dependency result: {}", e)))?
        .is_some();

    Ok(created)
}

/// Get all tasks that a given task depends on (prerequisites)
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `task_id` - ID of the task to get dependencies for
///
/// # Returns
/// * `Ok(Vec<String>)` - List of prerequisite task IDs
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::relationships::get_task_dependencies};
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
///     let dependencies = get_task_dependencies(client.graph(), "task-002").await?;
///     println!("Task depends on: {:?}", dependencies);
///     Ok(())
/// }
/// ```
pub async fn get_task_dependencies(graph: &Graph, task_id: &str) -> Result<Vec<String>> {
    let cypher = query(
        "MATCH (t:Task {id: $id})-[:DEPENDS_ON]->(prerequisite:Task)
         RETURN prerequisite.id as prereq_id",
    )
    .param("id", task_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to get task dependencies: {}", e))
    })?;

    let mut dependencies = Vec::new();

    while let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read dependency row: {}", e)))?
    {
        let prereq_id: String = row.get("prereq_id").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract prerequisite id: {}", e))
        })?;

        dependencies.push(prereq_id);
    }

    Ok(dependencies)
}

/// Get all tasks that depend on a given task (dependents)
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `task_id` - ID of the prerequisite task
///
/// # Returns
/// * `Ok(Vec<String>)` - List of dependent task IDs
/// * `Err(Neo4jError)` on failure
///
/// # Example
/// ```no_run
/// use ouroboros_kg::{Neo4jClient, schema::relationships::get_dependent_tasks};
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
///     let dependents = get_dependent_tasks(client.graph(), "task-001").await?;
///     println!("Tasks depending on this: {:?}", dependents);
///     Ok(())
/// }
/// ```
pub async fn get_dependent_tasks(graph: &Graph, task_id: &str) -> Result<Vec<String>> {
    let cypher = query(
        "MATCH (dependent:Task)-[:DEPENDS_ON]->(t:Task {id: $id})
         RETURN dependent.id as dep_id",
    )
    .param("id", task_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to get dependent tasks: {}", e))
    })?;

    let mut dependents = Vec::new();

    while let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read dependent row: {}", e)))?
    {
        let dep_id: String = row.get("dep_id").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract dependent id: {}", e))
        })?;

        dependents.push(dep_id);
    }

    Ok(dependents)
}

/// Remove a dependency relationship between two tasks
///
/// # Arguments
/// * `graph` - Neo4j graph connection
/// * `dependent_task_id` - ID of the dependent task
/// * `prerequisite_task_id` - ID of the prerequisite task
///
/// # Returns
/// * `Ok(bool)` - true if relationship existed and was removed
/// * `Err(Neo4jError)` on failure
pub async fn remove_dependency(
    graph: &Graph,
    dependent_task_id: &str,
    prerequisite_task_id: &str,
) -> Result<bool> {
    let cypher = query(
        "MATCH (dependent:Task {id: $dependent_id})-[r:DEPENDS_ON]->(prerequisite:Task {id: $prerequisite_id})
         DELETE r
         RETURN count(r) as deleted_count",
    )
    .param("dependent_id", dependent_task_id.to_string())
    .param("prerequisite_id", prerequisite_task_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to remove dependency: {}", e))
    })?;

    if let Some(row) = result
        .next()
        .await
        .map_err(|e| Neo4jError::QueryError(format!("Failed to read removal result: {}", e)))?
    {
        let deleted_count: i64 = row.get("deleted_count").map_err(|e| {
            Neo4jError::QueryError(format!("Failed to extract deleted count: {}", e))
        })?;

        Ok(deleted_count > 0)
    } else {
        Ok(false)
    }
}

// ============================================================================
// Knowledge Relationships
// ============================================================================

/// Create a CAUSED relationship between knowledge nodes
///
/// Creates: (cause:Knowledge)-[:CAUSED {reason}]->(effect:Knowledge)
/// Example: (failure-001)-[:CAUSED]->(decision-002: "LanceDB 전환")
pub async fn create_caused(
    graph: &Graph,
    cause_id: &str,
    effect_id: &str,
    reason: Option<&str>,
) -> Result<bool> {
    let cypher = if let Some(r) = reason {
        query(
            "MATCH (cause:Knowledge {id: $cause_id})
             MATCH (effect:Knowledge {id: $effect_id})
             MERGE (cause)-[:CAUSED {reason: $reason}]->(effect)
             RETURN cause, effect",
        )
        .param("cause_id", cause_id.to_string())
        .param("effect_id", effect_id.to_string())
        .param("reason", r.to_string())
    } else {
        query(
            "MATCH (cause:Knowledge {id: $cause_id})
             MATCH (effect:Knowledge {id: $effect_id})
             MERGE (cause)-[:CAUSED]->(effect)
             RETURN cause, effect",
        )
        .param("cause_id", cause_id.to_string())
        .param("effect_id", effect_id.to_string())
    };

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to create CAUSED relationship: {}", e))
    })?;

    Ok(result.next().await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to read result: {}", e))
    })?.is_some())
}

/// Create a SUPPORTS relationship (experiment confirms hypothesis)
///
/// Creates: (experiment:Knowledge)-[:SUPPORTS {confidence, metric}]->(decision:Knowledge)
pub async fn create_supports(
    graph: &Graph,
    experiment_id: &str,
    decision_id: &str,
    confidence: Option<f64>,
    metric: Option<&str>,
) -> Result<bool> {
    let mut params = vec![
        ("experiment_id", experiment_id.to_string()),
        ("decision_id", decision_id.to_string()),
    ];

    let query_str = match (confidence, metric) {
        (Some(c), Some(m)) => {
            params.push(("confidence", c.to_string()));
            params.push(("metric", m.to_string()));
            "MATCH (exp:Knowledge {id: $experiment_id})
             MATCH (dec:Knowledge {id: $decision_id})
             MERGE (exp)-[:SUPPORTS {confidence: toFloat($confidence), metric: $metric}]->(dec)
             RETURN exp, dec"
        }
        _ => {
            "MATCH (exp:Knowledge {id: $experiment_id})
             MATCH (dec:Knowledge {id: $decision_id})
             MERGE (exp)-[:SUPPORTS]->(dec)
             RETURN exp, dec"
        }
    };

    let mut cypher = query(query_str);
    for (key, val) in params {
        cypher = cypher.param(key, val);
    }

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to create SUPPORTS relationship: {}", e))
    })?;

    Ok(result.next().await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to read result: {}", e))
    })?.is_some())
}

/// Create a REFUTES relationship (experiment disproves hypothesis)
///
/// Creates: (experiment:Knowledge)-[:REFUTES {reason}]->(hypothesis:Knowledge)
pub async fn create_refutes(
    graph: &Graph,
    experiment_id: &str,
    hypothesis_id: &str,
    reason: Option<&str>,
) -> Result<bool> {
    let cypher = if let Some(r) = reason {
        query(
            "MATCH (exp:Knowledge {id: $experiment_id})
             MATCH (hyp:Knowledge {id: $hypothesis_id})
             MERGE (exp)-[:REFUTES {reason: $reason}]->(hyp)
             RETURN exp, hyp",
        )
        .param("experiment_id", experiment_id.to_string())
        .param("hypothesis_id", hypothesis_id.to_string())
        .param("reason", r.to_string())
    } else {
        query(
            "MATCH (exp:Knowledge {id: $experiment_id})
             MATCH (hyp:Knowledge {id: $hypothesis_id})
             MERGE (exp)-[:REFUTES]->(hyp)
             RETURN exp, hyp",
        )
        .param("experiment_id", experiment_id.to_string())
        .param("hypothesis_id", hypothesis_id.to_string())
    };

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to create REFUTES relationship: {}", e))
    })?;

    Ok(result.next().await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to read result: {}", e))
    })?.is_some())
}

/// Create a REPLACED_BY relationship (abandoned approach replaced by new one)
///
/// Creates: (abandoned:Knowledge)-[:REPLACED_BY {reason}]->(replacement:Knowledge)
pub async fn create_replaced_by(
    graph: &Graph,
    abandoned_id: &str,
    replacement_id: &str,
    reason: Option<&str>,
) -> Result<bool> {
    let cypher = if let Some(r) = reason {
        query(
            "MATCH (old:Knowledge {id: $abandoned_id})
             MATCH (new:Knowledge {id: $replacement_id})
             MERGE (old)-[:REPLACED_BY {reason: $reason}]->(new)
             RETURN old, new",
        )
        .param("abandoned_id", abandoned_id.to_string())
        .param("replacement_id", replacement_id.to_string())
        .param("reason", r.to_string())
    } else {
        query(
            "MATCH (old:Knowledge {id: $abandoned_id})
             MATCH (new:Knowledge {id: $replacement_id})
             MERGE (old)-[:REPLACED_BY]->(new)
             RETURN old, new",
        )
        .param("abandoned_id", abandoned_id.to_string())
        .param("replacement_id", replacement_id.to_string())
    };

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to create REPLACED_BY relationship: {}", e))
    })?;

    Ok(result.next().await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to read result: {}", e))
    })?.is_some())
}

/// Create a LEARNED_FROM relationship (learning derived from failure/experiment)
///
/// Creates: (learning:Knowledge)-[:LEARNED_FROM]->(source:Knowledge)
pub async fn create_learned_from(
    graph: &Graph,
    learning_id: &str,
    source_id: &str,
) -> Result<bool> {
    let cypher = query(
        "MATCH (learning:Knowledge {id: $learning_id})
         MATCH (source:Knowledge {id: $source_id})
         MERGE (learning)-[:LEARNED_FROM]->(source)
         RETURN learning, source",
    )
    .param("learning_id", learning_id.to_string())
    .param("source_id", source_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to create LEARNED_FROM relationship: {}", e))
    })?;

    Ok(result.next().await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to read result: {}", e))
    })?.is_some())
}

/// Query knowledge nodes related to a decision through any relationship
pub async fn get_related_knowledge(
    graph: &Graph,
    knowledge_id: &str,
) -> Result<Vec<(String, String, String)>> {
    let cypher = query(
        "MATCH (k:Knowledge {id: $id})-[r]-(related:Knowledge)
         RETURN related.id as related_id, type(r) as rel_type, related.title as title",
    )
    .param("id", knowledge_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to query related knowledge: {}", e))
    })?;

    let mut related = Vec::new();

    while let Some(row) = result.next().await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to read row: {}", e))
    })? {
        let related_id: String = row.get("related_id").unwrap_or_default();
        let rel_type: String = row.get("rel_type").unwrap_or_default();
        let title: String = row.get("title").unwrap_or_default();
        related.push((related_id, rel_type, title));
    }

    Ok(related)
}

/// Get the causal chain leading to a decision
///
/// Traverses CAUSED relationships backwards to find root causes
pub async fn get_causal_chain(
    graph: &Graph,
    knowledge_id: &str,
    max_depth: u32,
) -> Result<Vec<String>> {
    let cypher = query(&format!(
        "MATCH path = (root:Knowledge)-[:CAUSED*1..{}]->(k:Knowledge {{id: $id}})
         RETURN [n IN nodes(path) | n.id] as chain
         ORDER BY length(path) DESC
         LIMIT 1",
        max_depth
    ))
    .param("id", knowledge_id.to_string());

    let mut result = graph.execute(cypher).await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to query causal chain: {}", e))
    })?;

    if let Some(row) = result.next().await.map_err(|e| {
        Neo4jError::QueryError(format!("Failed to read result: {}", e))
    })? {
        let chain: Vec<String> = row.get("chain").unwrap_or_default();
        Ok(chain)
    } else {
        Ok(vec![knowledge_id.to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exists() {
        // Basic module test
        assert!(true);
    }
}
