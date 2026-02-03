use std::collections::HashMap;
use std::path::Path;
use std::fs;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use anyhow::{Result, anyhow, Context};
use serde::{Deserialize, Serialize};

use super::task::{Task, TaskStatus};

/// Edge type in the DAG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Regular dependency: source must complete before target starts (sequential)
    Dependency,
    /// Fork edge: source completion triggers parallel execution of targets
    Fork,
}

impl Default for EdgeType {
    fn default() -> Self {
        EdgeType::Dependency
    }
}

pub struct DagManager {
    graph: DiGraph<String, EdgeType>,  // Node = task_id, Edge = relationship type
    tasks: HashMap<String, Task>,
    indices: HashMap<String, NodeIndex>,
}

impl DagManager {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            tasks: HashMap::new(),
            indices: HashMap::new(),
        }
    }

    /// Add a task to the DAG
    pub fn add_task(&mut self, task: Task) -> Result<()> {
        if self.tasks.contains_key(&task.id) {
            return Err(anyhow!("Task {} already exists", task.id));
        }

        let idx = self.graph.add_node(task.id.clone());
        self.indices.insert(task.id.clone(), idx);
        self.tasks.insert(task.id.clone(), task);

        Ok(())
    }

    /// Add dependency: `task_id` depends on `depends_on_id`
    pub fn add_dependency(&mut self, task_id: &str, depends_on_id: &str) -> Result<()> {
        self.add_edge(depends_on_id, task_id, EdgeType::Dependency)
    }

    /// Add an edge with specific type
    fn add_edge(&mut self, from_id: &str, to_id: &str, edge_type: EdgeType) -> Result<()> {
        let from_idx = self.indices.get(from_id)
            .ok_or_else(|| anyhow!("Task {} not found", from_id))?;
        let to_idx = self.indices.get(to_id)
            .ok_or_else(|| anyhow!("Task {} not found", to_id))?;

        // Edge direction: from -> to
        self.graph.add_edge(*from_idx, *to_idx, edge_type);

        Ok(())
    }

    /// Fork: after `source` completes, all `targets` can run in parallel
    pub fn fork(&mut self, source: &str, targets: &[&str]) -> Result<()> {
        for target in targets {
            self.add_edge(source, target, EdgeType::Fork)?;
        }
        Ok(())
    }

    /// Join: `target` starts only after all `sources` complete
    pub fn join(&mut self, sources: &[&str], target: &str) -> Result<()> {
        for source in sources {
            self.add_edge(source, target, EdgeType::Dependency)?;
        }
        Ok(())
    }

    /// Get all fork points (tasks that have outgoing fork edges)
    pub fn fork_points(&self) -> Vec<&Task> {
        self.tasks.values()
            .filter(|task| {
                if let Some(&idx) = self.indices.get(&task.id) {
                    self.graph
                        .edges_directed(idx, Direction::Outgoing)
                        .any(|edge| *edge.weight() == EdgeType::Fork)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Get parallel branches from a fork point
    pub fn parallel_branches(&self, fork_task_id: &str) -> Vec<&str> {
        let Some(&idx) = self.indices.get(fork_task_id) else {
            return vec![];
        };

        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .filter(|edge| *edge.weight() == EdgeType::Fork)
            .filter_map(|edge| {
                self.graph.node_weight(edge.target())
                    .map(|s| s.as_str())
            })
            .collect()
    }

    /// Get edge type between two tasks
    pub fn edge_type(&self, from_id: &str, to_id: &str) -> Option<EdgeType> {
        let from_idx = self.indices.get(from_id)?;
        let to_idx = self.indices.get(to_id)?;

        self.graph.find_edge(*from_idx, *to_idx)
            .and_then(|edge_idx| self.graph.edge_weight(edge_idx).copied())
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Get mutable task by ID
    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(task_id)
    }

    /// Get all tasks
    pub fn tasks(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }

    /// Get tasks that are ready to execute (all dependencies completed)
    pub fn ready_tasks(&self) -> Vec<&Task> {
        self.tasks.values()
            .filter(|task| {
                if !task.is_pending() {
                    return false;
                }

                // Check if all dependencies are completed
                if let Some(&idx) = self.indices.get(&task.id) {
                    let deps_completed = self.graph
                        .neighbors_directed(idx, Direction::Incoming)
                        .all(|dep_idx| {
                            if let Some(dep_id) = self.graph.node_weight(dep_idx) {
                                self.tasks.get(dep_id)
                                    .map(|t| t.is_completed())
                                    .unwrap_or(false)
                            } else {
                                false
                            }
                        });
                    deps_completed
                } else {
                    false
                }
            })
            .collect()
    }

    /// Get topologically sorted execution order
    pub fn execution_order(&self) -> Result<Vec<String>> {
        let sorted = toposort(&self.graph, None)
            .map_err(|_| anyhow!("Cycle detected in task graph"))?;

        Ok(sorted.into_iter()
            .filter_map(|idx| self.graph.node_weight(idx).cloned())
            .collect())
    }

    /// Update task status
    pub fn update_status(&mut self, task_id: &str, status: TaskStatus) -> Result<()> {
        let task = self.tasks.get_mut(task_id)
            .ok_or_else(|| anyhow!("Task {} not found", task_id))?;
        task.status = status;
        Ok(())
    }

    /// Get dependencies of a task
    pub fn dependencies(&self, task_id: &str) -> Vec<&Task> {
        let Some(&idx) = self.indices.get(task_id) else {
            return vec![];
        };

        self.graph
            .neighbors_directed(idx, Direction::Incoming)
            .filter_map(|dep_idx| {
                self.graph.node_weight(dep_idx)
                    .and_then(|id| self.tasks.get(id))
            })
            .collect()
    }

    /// Get tasks blocked by this task
    pub fn blocked_by(&self, task_id: &str) -> Vec<&Task> {
        let Some(&idx) = self.indices.get(task_id) else {
            return vec![];
        };

        self.graph
            .neighbors_directed(idx, Direction::Outgoing)
            .filter_map(|blocked_idx| {
                self.graph.node_weight(blocked_idx)
                    .and_then(|id| self.tasks.get(id))
            })
            .collect()
    }

    /// Check if all tasks are done
    pub fn is_complete(&self) -> bool {
        self.tasks.values().all(|t| t.is_done())
    }

    /// Get completion statistics
    pub fn stats(&self) -> DagStats {
        let total = self.tasks.len();
        let completed = self.tasks.values().filter(|t| t.is_completed()).count();
        let failed = self.tasks.values().filter(|t| t.is_failed()).count();
        let pending = self.tasks.values().filter(|t| t.is_pending()).count();
        let in_progress = self.tasks.values()
            .filter(|t| matches!(t.status, TaskStatus::InProgress))
            .count();

        DagStats { total, completed, failed, pending, in_progress }
    }
}

impl Default for DagManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Edge relationship for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Source task (from)
    pub from: String,
    /// Target task (to)
    pub to: String,
    /// Type of relationship
    #[serde(default)]
    pub edge_type: EdgeType,
}

/// Serializable DAG state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagState {
    pub tasks: Vec<Task>,
    pub edges: Vec<Edge>,
}

impl DagManager {
    /// Save DAG to JSON file
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut edges = vec![];

        for edge_ref in self.graph.edge_references() {
            let from_id = self.graph.node_weight(edge_ref.source())
                .ok_or_else(|| anyhow!("Invalid edge source"))?;
            let to_id = self.graph.node_weight(edge_ref.target())
                .ok_or_else(|| anyhow!("Invalid edge target"))?;

            edges.push(Edge {
                from: from_id.clone(),
                to: to_id.clone(),
                edge_type: *edge_ref.weight(),
            });
        }

        let state = DagState {
            tasks: self.tasks.values().cloned().collect(),
            edges,
        };

        let json = serde_json::to_string_pretty(&state)?;
        fs::write(path, json).context("Failed to write DAG state")?;

        Ok(())
    }

    /// Load DAG from JSON file
    pub fn load(path: &Path) -> Result<Self> {
        let json = fs::read_to_string(path).context("Failed to read DAG state")?;
        let state: DagState = serde_json::from_str(&json)?;

        let mut dag = Self::new();

        for task in state.tasks {
            dag.add_task(task)?;
        }

        for edge in state.edges {
            dag.add_edge(&edge.from, &edge.to, edge.edge_type)?;
        }

        Ok(dag)
    }

    /// Check if DAG file exists
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }
}

#[derive(Debug, Clone)]
pub struct DagStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub in_progress: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_basic() {
        let mut dag = DagManager::new();

        let task1 = Task::new("Task 1", "First task").with_id("task-1");
        let task2 = Task::new("Task 2", "Second task").with_id("task-2");
        let task3 = Task::new("Task 3", "Third task").with_id("task-3");

        dag.add_task(task1).unwrap();
        dag.add_task(task2).unwrap();
        dag.add_task(task3).unwrap();

        // task-2 depends on task-1
        // task-3 depends on task-2
        dag.add_dependency("task-2", "task-1").unwrap();
        dag.add_dependency("task-3", "task-2").unwrap();

        // Only task-1 should be ready
        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "task-1");

        // Complete task-1
        dag.get_task_mut("task-1").unwrap().complete(None);

        // Now task-2 should be ready
        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "task-2");
    }

    #[test]
    fn test_execution_order() {
        let mut dag = DagManager::new();

        dag.add_task(Task::new("A", "").with_id("a")).unwrap();
        dag.add_task(Task::new("B", "").with_id("b")).unwrap();
        dag.add_task(Task::new("C", "").with_id("c")).unwrap();

        dag.add_dependency("b", "a").unwrap();
        dag.add_dependency("c", "b").unwrap();

        let order = dag.execution_order().unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_fork() {
        let mut dag = DagManager::new();

        // Create tasks: start -> (branch1, branch2, branch3) -> merge
        dag.add_task(Task::new("Start", "").with_id("start")).unwrap();
        dag.add_task(Task::new("Branch 1", "").with_id("branch-1")).unwrap();
        dag.add_task(Task::new("Branch 2", "").with_id("branch-2")).unwrap();
        dag.add_task(Task::new("Branch 3", "").with_id("branch-3")).unwrap();
        dag.add_task(Task::new("Merge", "").with_id("merge")).unwrap();

        // Fork from start to branches
        dag.fork("start", &["branch-1", "branch-2", "branch-3"]).unwrap();

        // Join branches to merge
        dag.join(&["branch-1", "branch-2", "branch-3"], "merge").unwrap();

        // Check fork points
        let fork_points = dag.fork_points();
        assert_eq!(fork_points.len(), 1);
        assert_eq!(fork_points[0].id, "start");

        // Check parallel branches
        let branches = dag.parallel_branches("start");
        assert_eq!(branches.len(), 3);
        assert!(branches.contains(&"branch-1"));
        assert!(branches.contains(&"branch-2"));
        assert!(branches.contains(&"branch-3"));

        // Check edge types
        assert_eq!(dag.edge_type("start", "branch-1"), Some(EdgeType::Fork));
        assert_eq!(dag.edge_type("branch-1", "merge"), Some(EdgeType::Dependency));

        // Only start should be ready initially
        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "start");

        // Complete start
        dag.get_task_mut("start").unwrap().complete(None);

        // All branches should be ready now (parallel)
        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 3);

        // Complete all branches
        dag.get_task_mut("branch-1").unwrap().complete(None);
        dag.get_task_mut("branch-2").unwrap().complete(None);
        dag.get_task_mut("branch-3").unwrap().complete(None);

        // Merge should be ready
        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "merge");
    }

    #[test]
    fn test_fork_partial_completion() {
        let mut dag = DagManager::new();

        dag.add_task(Task::new("Start", "").with_id("start")).unwrap();
        dag.add_task(Task::new("Branch 1", "").with_id("branch-1")).unwrap();
        dag.add_task(Task::new("Branch 2", "").with_id("branch-2")).unwrap();
        dag.add_task(Task::new("Merge", "").with_id("merge")).unwrap();

        dag.fork("start", &["branch-1", "branch-2"]).unwrap();
        dag.join(&["branch-1", "branch-2"], "merge").unwrap();

        dag.get_task_mut("start").unwrap().complete(None);

        // Complete only branch-1
        dag.get_task_mut("branch-1").unwrap().complete(None);

        // Merge should NOT be ready (branch-2 still pending)
        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "branch-2");

        // Complete branch-2
        dag.get_task_mut("branch-2").unwrap().complete(None);

        // Now merge should be ready
        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "merge");
    }
}
