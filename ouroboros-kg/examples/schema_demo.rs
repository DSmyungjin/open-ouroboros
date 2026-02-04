//! Demonstrates the knowledge graph schema usage
//!
//! This example shows how to:
//! - Create tasks, results, and contexts
//! - Link them together with relationships
//! - Query the graph

use ouroboros_kg::schema::{
    create_context, create_dependency, create_result, create_task, get_task, link_context_to_result,
    link_result_to_task, update_task, Context, Task, TaskResult, TaskStatus,
};
use ouroboros_kg::Neo4jClient;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Connect to Neo4j
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());
    let database = std::env::var("NEO4J_DATABASE").unwrap_or_else(|_| "neo4j".to_string());

    println!("Connecting to Neo4j at {}...", uri);
    let client = Neo4jClient::new(&uri, &user, &password, &database).await?;

    println!("✓ Connected to Neo4j\n");

    // 1. Create tasks
    println!("1. Creating tasks...");

    let task1 = Task::new(
        "schema-demo-001".to_string(),
        "Setup Database".to_string(),
        "Initialize and configure Neo4j database".to_string(),
    );
    create_task(client.graph(), &task1).await?;
    println!("   Created task: {} - {}", task1.id, task1.subject);

    let task2 = Task::new(
        "schema-demo-002".to_string(),
        "Load Initial Data".to_string(),
        "Load seed data into the database".to_string(),
    );
    create_task(client.graph(), &task2).await?;
    println!("   Created task: {} - {}", task2.id, task2.subject);

    let task3 = Task::new(
        "schema-demo-003".to_string(),
        "Run Tests".to_string(),
        "Execute integration tests".to_string(),
    );
    create_task(client.graph(), &task3).await?;
    println!("   Created task: {} - {}\n", task3.id, task3.subject);

    // 2. Create dependencies
    println!("2. Creating task dependencies...");

    // task2 depends on task1
    let dep1 = create_dependency(client.graph(), "schema-demo-002", "schema-demo-001").await?;
    println!("   {} -> {} depends on {} -> {}",
        task2.id, task2.subject, task1.id, task1.subject);

    // task3 depends on task2
    let dep2 = create_dependency(client.graph(), "schema-demo-003", "schema-demo-002").await?;
    println!("   {} -> {} depends on {} -> {}\n",
        task3.id, task3.subject, task2.id, task2.subject);

    if !dep1 || !dep2 {
        println!("   ⚠ Warning: Some dependencies could not be created");
    }

    // 3. Update task status
    println!("3. Updating task statuses...");
    update_task(client.graph(), "schema-demo-001", TaskStatus::Completed).await?;
    println!("   Updated {} to Completed", task1.id);

    update_task(client.graph(), "schema-demo-002", TaskStatus::InProgress).await?;
    println!("   Updated {} to InProgress\n", task2.id);

    // 4. Retrieve task
    println!("4. Retrieving task...");
    if let Some(retrieved_task) = get_task(client.graph(), "schema-demo-001").await? {
        println!("   Retrieved: {}", retrieved_task.subject);
        println!("   Status: {:?}", retrieved_task.status);
        println!("   Created: {}\n", retrieved_task.created_at);
    }

    // 5. Create results
    println!("5. Creating results...");

    let result1 = TaskResult::new(
        "schema-demo-result-001".to_string(),
        "Database successfully initialized with schema v1.0".to_string(),
    );
    create_result(client.graph(), &result1).await?;
    println!("   Created result: {}", result1.id);

    // Link result to task
    let linked = link_result_to_task(client.graph(), "schema-demo-001", "schema-demo-result-001").await?;
    if linked {
        println!("   Linked result to task {}\n", task1.id);
    } else {
        println!("   ⚠ Failed to link result to task\n");
    }

    // 6. Create contexts
    println!("6. Creating contexts...");

    let context1 = Context::with_metadata(
        "schema-demo-ctx-001".to_string(),
        "Neo4j Documentation".to_string(),
        json!({
            "version": "5.0",
            "url": "https://neo4j.com/docs",
            "topic": "schema design"
        }),
    );
    create_context(client.graph(), &context1).await?;
    println!("   Created context: {}", context1.name);

    // Link context to result
    let linked = link_context_to_result(
        client.graph(),
        "schema-demo-result-001",
        "schema-demo-ctx-001",
    )
    .await?;
    if linked {
        println!("   Linked context to result\n");
    } else {
        println!("   ⚠ Failed to link context to result\n");
    }

    // 7. Summary
    println!("{}", "=".repeat(50));
    println!("Schema Demo Complete!");
    println!("{}", "=".repeat(50));
    println!("Created:");
    println!("  - 3 Task nodes");
    println!("  - 2 DEPENDS_ON relationships");
    println!("  - 1 Result node");
    println!("  - 1 PRODUCED relationship (Task->Result)");
    println!("  - 1 Context node");
    println!("  - 1 REFERENCES relationship (Result->Context)");
    println!("\nGraph structure:");
    println!("  Task[{}] -PRODUCED-> Result[{}] -REFERENCES-> Context[{}]",
        task1.id, result1.id, context1.id);
    println!("  Task[{}] -DEPENDS_ON-> Task[{}]", task2.id, task1.id);
    println!("  Task[{}] -DEPENDS_ON-> Task[{}]", task3.id, task2.id);
    println!("\nYou can visualize this in Neo4j Browser with:");
    println!("  MATCH (n) WHERE n.id STARTS WITH 'schema-demo' RETURN n");

    Ok(())
}
