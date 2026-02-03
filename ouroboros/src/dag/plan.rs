use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

use super::manager::{DagManager, EdgeType};
use super::task::Task;
use super::context::ContextTree;

/// Planning agent's output - complete execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Workflow DAG specification
    pub workflow: WorkflowSpec,
    /// Context tree design
    pub context_design: ContextDesign,
}

/// Workflow DAG specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSpec {
    pub tasks: Vec<TaskSpec>,
    pub edges: Vec<EdgeSpec>,
}

/// Task specification from planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: String,
    pub subject: String,
    pub description: String,
}

/// Edge specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSpec {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

/// Context tree design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDesign {
    /// Fork point configurations
    pub fork_configs: Vec<ForkConfig>,
}

/// Configuration for a single fork point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkConfig {
    /// Task that triggers this fork (fork happens after this task completes)
    pub trigger_task: String,
    /// Documents to cache as shared context before forking
    pub shared_context: Vec<String>,
    /// Branch configurations
    pub branches: Vec<BranchConfig>,
}

/// Configuration for a single branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchConfig {
    /// Corresponding workflow task ID
    pub task_id: String,
    /// Expected output document types/paths
    pub expected_outputs: Vec<String>,
}

impl ExecutionPlan {
    /// Create an empty execution plan
    pub fn new() -> Self {
        Self {
            workflow: WorkflowSpec {
                tasks: vec![],
                edges: vec![],
            },
            context_design: ContextDesign {
                fork_configs: vec![],
            },
        }
    }

    /// Build DagManager from workflow spec
    pub fn build_dag(&self) -> Result<DagManager> {
        let mut dag = DagManager::new();

        // Add tasks
        for task_spec in &self.workflow.tasks {
            let task = Task::new(&task_spec.subject, &task_spec.description)
                .with_id(&task_spec.id);
            dag.add_task(task)?;
        }

        // Add edges
        for edge in &self.workflow.edges {
            match edge.edge_type {
                EdgeType::Dependency => {
                    dag.add_dependency(&edge.to, &edge.from)?;
                }
                EdgeType::Fork => {
                    dag.fork(&edge.from, &[edge.to.as_str()])?;
                }
            }
        }

        Ok(dag)
    }

    /// Initialize ContextTree based on context design
    pub fn init_context_tree(&self) -> ContextTree {
        let mut tree = ContextTree::new();
        tree.init_root();
        tree
    }

    /// Get fork config for a specific trigger task
    pub fn fork_config_for(&self, task_id: &str) -> Option<&ForkConfig> {
        self.context_design.fork_configs.iter()
            .find(|fc| fc.trigger_task == task_id)
    }

    /// Get all fork trigger tasks
    pub fn fork_triggers(&self) -> Vec<&str> {
        self.context_design.fork_configs.iter()
            .map(|fc| fc.trigger_task.as_str())
            .collect()
    }

    /// Validate plan consistency
    pub fn validate(&self) -> Result<()> {
        let task_ids: std::collections::HashSet<_> =
            self.workflow.tasks.iter().map(|t| t.id.as_str()).collect();

        // Check edges reference valid tasks
        for edge in &self.workflow.edges {
            if !task_ids.contains(edge.from.as_str()) {
                return Err(anyhow!("Edge references unknown task: {}", edge.from));
            }
            if !task_ids.contains(edge.to.as_str()) {
                return Err(anyhow!("Edge references unknown task: {}", edge.to));
            }
        }

        // Check fork configs reference valid tasks
        for fork_config in &self.context_design.fork_configs {
            if !task_ids.contains(fork_config.trigger_task.as_str()) {
                return Err(anyhow!(
                    "Fork config references unknown trigger task: {}",
                    fork_config.trigger_task
                ));
            }
            for branch in &fork_config.branches {
                if !task_ids.contains(branch.task_id.as_str()) {
                    return Err(anyhow!(
                        "Branch references unknown task: {}",
                        branch.task_id
                    ));
                }
            }
        }

        // Check fork edges match fork configs
        let fork_edges: Vec<_> = self.workflow.edges.iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Fork))
            .collect();

        for fork_config in &self.context_design.fork_configs {
            let branch_tasks: std::collections::HashSet<_> =
                fork_config.branches.iter().map(|b| b.task_id.as_str()).collect();

            let edges_from_trigger: std::collections::HashSet<_> = fork_edges.iter()
                .filter(|e| e.from == fork_config.trigger_task)
                .map(|e| e.to.as_str())
                .collect();

            if branch_tasks != edges_from_trigger {
                return Err(anyhow!(
                    "Fork config branches don't match fork edges for task: {}",
                    fork_config.trigger_task
                ));
            }
        }

        Ok(())
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating execution plans programmatically
pub struct PlanBuilder {
    plan: ExecutionPlan,
}

impl PlanBuilder {
    pub fn new() -> Self {
        Self {
            plan: ExecutionPlan::new(),
        }
    }

    /// Add a task
    pub fn task(mut self, id: &str, subject: &str, description: &str) -> Self {
        self.plan.workflow.tasks.push(TaskSpec {
            id: id.to_string(),
            subject: subject.to_string(),
            description: description.to_string(),
        });
        self
    }

    /// Add a dependency edge
    pub fn dependency(mut self, from: &str, to: &str) -> Self {
        self.plan.workflow.edges.push(EdgeSpec {
            from: from.to_string(),
            to: to.to_string(),
            edge_type: EdgeType::Dependency,
        });
        self
    }

    /// Add a fork configuration with edges
    pub fn fork(mut self, trigger: &str, branches: &[(&str, Vec<&str>)], shared_context: Vec<&str>) -> Self {
        let mut branch_configs = vec![];

        for (task_id, expected_outputs) in branches {
            // Add fork edge
            self.plan.workflow.edges.push(EdgeSpec {
                from: trigger.to_string(),
                to: task_id.to_string(),
                edge_type: EdgeType::Fork,
            });

            // Add branch config
            branch_configs.push(BranchConfig {
                task_id: task_id.to_string(),
                expected_outputs: expected_outputs.iter().map(|s| s.to_string()).collect(),
            });
        }

        self.plan.context_design.fork_configs.push(ForkConfig {
            trigger_task: trigger.to_string(),
            shared_context: shared_context.iter().map(|s| s.to_string()).collect(),
            branches: branch_configs,
        });

        self
    }

    /// Add a join (multiple dependencies to one target)
    pub fn join(mut self, sources: &[&str], target: &str) -> Self {
        for source in sources {
            self.plan.workflow.edges.push(EdgeSpec {
                from: source.to_string(),
                to: target.to_string(),
                edge_type: EdgeType::Dependency,
            });
        }
        self
    }

    /// Build and validate the plan
    pub fn build(self) -> Result<ExecutionPlan> {
        self.plan.validate()?;
        Ok(self.plan)
    }

    /// Build without validation (for testing)
    pub fn build_unchecked(self) -> ExecutionPlan {
        self.plan
    }
}

impl Default for PlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_builder() {
        let plan = PlanBuilder::new()
            .task("analyze", "Analyze code", "Analyze the codebase structure")
            .task("write-tests", "Write tests", "Write unit tests")
            .task("write-docs", "Write docs", "Write documentation")
            .task("merge", "Merge results", "Merge all outputs")
            .fork(
                "analyze",
                &[
                    ("write-tests", vec!["test_plan.md", "tests/"]),
                    ("write-docs", vec!["api_docs.md", "readme.md"]),
                ],
                vec!["project_structure.md", "analysis_result.md"],
            )
            .join(&["write-tests", "write-docs"], "merge")
            .build()
            .unwrap();

        assert_eq!(plan.workflow.tasks.len(), 4);
        assert_eq!(plan.workflow.edges.len(), 4); // 2 fork + 2 join
        assert_eq!(plan.context_design.fork_configs.len(), 1);

        // Check fork config
        let fork_config = &plan.context_design.fork_configs[0];
        assert_eq!(fork_config.trigger_task, "analyze");
        assert_eq!(fork_config.branches.len(), 2);
        assert_eq!(fork_config.shared_context.len(), 2);
    }

    #[test]
    fn test_build_dag_from_plan() {
        let plan = PlanBuilder::new()
            .task("a", "Task A", "")
            .task("b", "Task B", "")
            .task("c", "Task C", "")
            .task("d", "Task D", "")
            .fork("a", &[("b", vec![]), ("c", vec![])], vec![])
            .join(&["b", "c"], "d")
            .build()
            .unwrap();

        let dag = plan.build_dag().unwrap();

        // Check tasks exist
        assert!(dag.get_task("a").is_some());
        assert!(dag.get_task("b").is_some());
        assert!(dag.get_task("c").is_some());
        assert!(dag.get_task("d").is_some());

        // Check fork
        let branches = dag.parallel_branches("a");
        assert_eq!(branches.len(), 2);
        assert!(branches.contains(&"b"));
        assert!(branches.contains(&"c"));

        // Check edge types
        assert_eq!(dag.edge_type("a", "b"), Some(EdgeType::Fork));
        assert_eq!(dag.edge_type("b", "d"), Some(EdgeType::Dependency));
    }

    #[test]
    fn test_validation_invalid_edge() {
        let plan = PlanBuilder::new()
            .task("a", "Task A", "")
            .dependency("a", "nonexistent")
            .build_unchecked();

        assert!(plan.validate().is_err());
    }

    #[test]
    fn test_validation_mismatched_fork() {
        let mut plan = PlanBuilder::new()
            .task("a", "Task A", "")
            .task("b", "Task B", "")
            .task("c", "Task C", "")
            .build_unchecked();

        // Add fork config that doesn't match edges
        plan.context_design.fork_configs.push(ForkConfig {
            trigger_task: "a".to_string(),
            shared_context: vec![],
            branches: vec![
                BranchConfig { task_id: "b".to_string(), expected_outputs: vec![] },
                BranchConfig { task_id: "c".to_string(), expected_outputs: vec![] },
            ],
        });

        // No fork edges added, so validation should fail
        assert!(plan.validate().is_err());
    }

    #[test]
    fn test_fork_triggers() {
        let plan = PlanBuilder::new()
            .task("start", "Start", "")
            .task("fork1-a", "Fork 1 A", "")
            .task("fork1-b", "Fork 1 B", "")
            .task("middle", "Middle", "")
            .task("fork2-a", "Fork 2 A", "")
            .task("fork2-b", "Fork 2 B", "")
            .task("end", "End", "")
            .fork("start", &[("fork1-a", vec![]), ("fork1-b", vec![])], vec![])
            .join(&["fork1-a", "fork1-b"], "middle")
            .fork("middle", &[("fork2-a", vec![]), ("fork2-b", vec![])], vec![])
            .join(&["fork2-a", "fork2-b"], "end")
            .build()
            .unwrap();

        let triggers = plan.fork_triggers();
        assert_eq!(triggers.len(), 2);
        assert!(triggers.contains(&"start"));
        assert!(triggers.contains(&"middle"));
    }
}
