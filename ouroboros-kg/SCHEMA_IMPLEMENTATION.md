# Knowledge Graph Schema Implementation

## Overview

This document describes the Neo4j knowledge graph schema implementation for storing task execution results in the Ouroboros system.

## Schema Design

### Node Types

#### 1. Task Node
Represents a unit of work in the system.

**Properties:**
- `id` (String): Unique identifier for the task
- `subject` (String): Title/subject of the task
- `description` (String): Detailed description of what the task does
- `status` (String): Current status - one of: `pending`, `in_progress`, `completed`, `failed`
- `created_at` (DateTime): Timestamp when the task was created

**Label:** `Task`

#### 2. Result Node
Represents the output or outcome of a task execution.

**Properties:**
- `id` (String): Unique identifier for the result
- `content` (String): The actual result content (can be text, JSON, etc.)
- `created_at` (DateTime): Timestamp when the result was created

**Label:** `Result`

#### 3. Context Node
Represents contextual information or resources referenced by results.

**Properties:**
- `id` (String): Unique identifier for the context
- `name` (String): Name/title of the context
- `metadata` (JSON String): Additional metadata as a JSON object

**Label:** `Context`

### Relationships

#### 1. PRODUCED
Connects a Task to its Result.

**Pattern:** `(Task)-[:PRODUCED]->(Result)`

**Semantics:** Indicates that a task produced this result when it was executed.

#### 2. REFERENCES
Connects a Result to Context nodes it references.

**Pattern:** `(Result)-[:REFERENCES]->(Context)`

**Semantics:** Indicates that a result references or uses this context information.

#### 3. DEPENDS_ON
Connects dependent tasks to their prerequisites.

**Pattern:** `(Task)-[:DEPENDS_ON]->(Task)`

**Semantics:** Indicates that the source task depends on the target task being completed first.

## Graph Structure Example

```
Task[task-001: "Setup DB"] -PRODUCED-> Result[result-001] -REFERENCES-> Context[ctx-001: "Documentation"]
                                                          \-REFERENCES-> Context[ctx-002: "Config"]
    ^
    |
    DEPENDS_ON
    |
Task[task-002: "Load Data"] -PRODUCED-> Result[result-002]
    ^
    |
    DEPENDS_ON
    |
Task[task-003: "Run Tests"]
```

## API Reference

### Task Operations

#### Create Task
```rust
use ouroboros_kg::schema::{Task, create_task};

let task = Task::new(
    "task-001".to_string(),
    "My Task".to_string(),
    "Task description".to_string()
);

create_task(client.graph(), &task).await?;
```

#### Get Task
```rust
use ouroboros_kg::schema::get_task;

let task = get_task(client.graph(), "task-001").await?;
if let Some(task) = task {
    println!("Found: {}", task.subject);
}
```

#### Update Task Status
```rust
use ouroboros_kg::schema::{update_task, TaskStatus};

update_task(client.graph(), "task-001", TaskStatus::Completed).await?;
```

### Result Operations

#### Create Result
```rust
use ouroboros_kg::schema::{Result as TaskResult, create_result};

let result = TaskResult::new(
    "result-001".to_string(),
    "Task completed successfully".to_string()
);

create_result(client.graph(), &result).await?;
```

#### Link Result to Task
```rust
use ouroboros_kg::schema::link_result_to_task;

link_result_to_task(client.graph(), "task-001", "result-001").await?;
```

### Context Operations

#### Create Context
```rust
use ouroboros_kg::schema::{Context, create_context};
use serde_json::json;

let context = Context::with_metadata(
    "ctx-001".to_string(),
    "API Documentation".to_string(),
    json!({"version": "1.0", "url": "https://example.com"})
);

create_context(client.graph(), &context).await?;
```

#### Link Context to Result
```rust
use ouroboros_kg::schema::link_context_to_result;

link_context_to_result(client.graph(), "result-001", "ctx-001").await?;
```

### Dependency Operations

#### Create Dependency
```rust
use ouroboros_kg::schema::create_dependency;

// task-002 depends on task-001
create_dependency(client.graph(), "task-002", "task-001").await?;
```

#### Get Task Dependencies
```rust
use ouroboros_kg::schema::relationships::get_task_dependencies;

let deps = get_task_dependencies(client.graph(), "task-002").await?;
// Returns: vec!["task-001"]
```

#### Get Dependent Tasks
```rust
use ouroboros_kg::schema::relationships::get_dependent_tasks;

let dependents = get_dependent_tasks(client.graph(), "task-001").await?;
// Returns: vec!["task-002"]
```

## Module Structure

```
src/schema/
├── mod.rs              # Module exports
├── types.rs            # Type definitions (Task, Result, Context, TaskStatus)
├── task.rs             # Task CRUD operations
├── result.rs           # Result CRUD operations
├── context.rs          # Context CRUD operations
└── relationships.rs    # Relationship operations (dependencies)
```

## Error Handling

All operations return `Result<T, Neo4jError>` where:
- `Ok(T)` indicates success
- `Err(Neo4jError)` contains detailed error information

Common error types:
- `Neo4jError::ConnectionError`: Network or connection issues
- `Neo4jError::QueryError`: Query execution failures
- `Neo4jError::SerializationError`: JSON serialization issues

## Testing

Run the schema tests:
```bash
cargo test --lib schema
```

Run the example:
```bash
# Set environment variables (optional)
export NEO4J_URI="bolt://localhost:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="password"
export NEO4J_DATABASE="neo4j"

# Run the demo
cargo run --example schema_demo
```

## Future Enhancements

1. **Query Operations**: Add functions to query the graph
   - Get all tasks with a specific status
   - Get task execution history
   - Get dependency chain for a task

2. **Batch Operations**: Add bulk insert/update functions
   - Create multiple tasks at once
   - Batch status updates

3. **Graph Traversal**: Add functions for graph analysis
   - Find execution path
   - Detect circular dependencies
   - Calculate task priority based on dependencies

4. **Indexing**: Add Neo4j constraints and indexes
   - Unique constraints on IDs
   - Index on task status
   - Full-text search on descriptions

5. **Validation**: Add schema validation
   - Prevent circular dependencies
   - Validate status transitions
   - Ensure prerequisite completion

## Performance Considerations

1. **Connection Pooling**: The `Neo4jClient` uses connection pooling (max 16 connections)
2. **Batch Operations**: For bulk inserts, consider using Neo4j's UNWIND clause
3. **Indexes**: Create indexes on frequently queried properties (id, status)
4. **Lazy Loading**: Consider pagination for large result sets

## Neo4j Queries

### View All Schema Demo Nodes
```cypher
MATCH (n) WHERE n.id STARTS WITH 'schema-demo' RETURN n
```

### View Full Graph
```cypher
MATCH (n) RETURN n LIMIT 100
```

### View Task Dependencies
```cypher
MATCH (dependent:Task)-[:DEPENDS_ON]->(prerequisite:Task)
RETURN dependent.id, prerequisite.id
```

### View Task Results with Context
```cypher
MATCH (task:Task)-[:PRODUCED]->(result:Result)-[:REFERENCES]->(context:Context)
RETURN task.subject, result.content, context.name
```

### Find All Completed Tasks
```cypher
MATCH (task:Task {status: 'completed'})
RETURN task.id, task.subject, task.created_at
ORDER BY task.created_at DESC
```

### Find Dependency Chain
```cypher
MATCH path = (start:Task {id: 'task-001'})<-[:DEPENDS_ON*]-(dependent:Task)
RETURN path
```
