use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

use crate::cli::{CliRunner, CliOptions, Model};
use crate::dag::{DagManager, DagStats, Task, ContextTree, ExecutionPlan};
use crate::docs::{DocumentStore, Document, DocType};

pub struct Orchestrator {
    cli: CliRunner,
    dag: DagManager,
    docs: DocumentStore,
    config: OrchestratorConfig,
    data_dir: PathBuf,
    context_tree: ContextTree,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub default_model: Model,
    pub validation_model: Model,
    pub extraction_model: Model,
    pub auto_validate: bool,
    /// Maximum retry attempts per task
    pub max_retries: u32,
    /// Number of validation checks to run
    pub validation_checks: u32,
    /// Minimum checks that must pass for validation success
    pub validation_threshold: u32,
    /// Threshold for post-fix recheck (can be lower than initial)
    pub recheck_threshold: u32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            default_model: Model::Sonnet,
            validation_model: Model::Opus,
            extraction_model: Model::Haiku,
            auto_validate: false,
            max_retries: 3,
            validation_checks: 3,
            validation_threshold: 2,  // 2 out of 3 must pass
            recheck_threshold: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub session_id: Option<String>,
}

impl Orchestrator {
    pub fn new(working_dir: PathBuf, data_dir: PathBuf) -> Result<Self> {
        let cli = CliRunner::new(working_dir.clone());
        let docs = DocumentStore::new(&data_dir)?;

        // Try to load existing DAG
        let dag_path = data_dir.join("dag.json");
        let dag = if DagManager::exists(&dag_path) {
            DagManager::load(&dag_path).unwrap_or_else(|_| DagManager::new())
        } else {
            DagManager::new()
        };

        // Try to load existing context tree
        let ctx_path = data_dir.join("context-tree.json");
        let context_tree = if ctx_path.exists() {
            let content = std::fs::read_to_string(&ctx_path).ok();
            content
                .and_then(|c| serde_json::from_str(&c).ok())
                .map(ContextTree::from_state)
                .unwrap_or_else(ContextTree::new)
        } else {
            ContextTree::new()
        };

        Ok(Self {
            cli,
            dag,
            docs,
            config: OrchestratorConfig::default(),
            data_dir,
            context_tree,
        })
    }

    pub fn with_config(mut self, config: OrchestratorConfig) -> Self {
        self.config = config;
        self
    }

    /// Save DAG state to file
    pub fn save_dag(&self) -> Result<()> {
        let dag_path = self.data_dir.join("dag.json");
        self.dag.save(&dag_path)
    }

    /// Save context tree to file
    pub fn save_context_tree(&self) -> Result<()> {
        let ctx_path = self.data_dir.join("context-tree.json");
        let state = self.context_tree.to_state();
        let content = serde_json::to_string_pretty(&state)?;
        std::fs::write(&ctx_path, content)?;
        Ok(())
    }

    /// Get context tree reference
    pub fn context_tree(&self) -> &ContextTree {
        &self.context_tree
    }

    /// Get mutable context tree reference
    pub fn context_tree_mut(&mut self) -> &mut ContextTree {
        &mut self.context_tree
    }

    /// Plan: Generate task DAG from a goal
    pub async fn plan(&mut self, goal: &str) -> Result<Vec<String>> {
        // Generate plan ID for session tracking
        let plan_id = format!("plan-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());

        let prompt = format!(
            r#"[PLAN:{}] Task Planning

You are a task planner. Break down this goal into concrete, actionable tasks.

Goal: {}

Output a JSON array of tasks. Each task should have:
- id: unique identifier (e.g., "task-001")
- subject: short title
- description: detailed description of what to do
- depends_on: array of task IDs this depends on (empty if none)

Example:
[
  {{"id": "task-001", "subject": "Analyze requirements", "description": "...", "depends_on": []}},
  {{"id": "task-002", "subject": "Design architecture", "description": "...", "depends_on": ["task-001"]}}
]

Output ONLY the JSON array, no other text."#,
            plan_id, goal
        );

        let options = CliOptions {
            model: self.config.default_model,
            print: true,
            ..Default::default()
        };

        let output = self.cli.run(&prompt, options).await?;

        // Save plan metadata
        let plan_meta = Document::new(
            &plan_id,
            DocType::Context,
            format!("# Plan: {}\n\nGoal: {}\n\nSession: {:?}", plan_id, goal, output.session_id),
        );
        self.docs.create(&plan_meta)?;

        // Parse the response
        let tasks: Vec<PlannedTask> = self.parse_task_json(&output.response)?;

        // Add tasks to DAG
        let mut task_ids = vec![];
        for planned in &tasks {
            let task = Task::new(&planned.subject, &planned.description)
                .with_id(&planned.id);
            self.dag.add_task(task)?;
            task_ids.push(planned.id.clone());

            // Save task definition
            let doc = Document::new(
                &planned.id,
                DocType::TaskDefinition,
                format!("# {}\n\n{}", planned.subject, planned.description),
            ).with_task_id(&planned.id);
            self.docs.create(&doc)?;
        }

        // Add dependencies
        for planned in &tasks {
            for dep_id in &planned.depends_on {
                self.dag.add_dependency(&planned.id, dep_id)?;
            }
        }

        // Save DAG to file
        self.save_dag()?;

        tracing::info!("Created {} tasks", task_ids.len());
        Ok(task_ids)
    }

    /// Execute a single task
    pub async fn execute_task(&mut self, task_id: &str) -> Result<TaskResult> {
        let task = self.dag.get_task(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
            .clone();

        let attempt_num = task.current_attempt();
        let task_type_str = if task.is_context_fill() { "ctx-fill" } else { "worker" };
        tracing::info!("Executing {} task: {} - {} (attempt #{})",
            task_type_str, task_id, task.subject, attempt_num);

        // Mark as in progress
        self.dag.get_task_mut(task_id).unwrap().start();

        // Get dependencies' results for context
        let dep_ids: Vec<String> = self.dag.dependencies(task_id)
            .iter()
            .map(|t| t.id.clone())
            .collect();

        // Assemble context from:
        // 1. Context tree docs (if context_ref is set)
        // 2. Dependency results
        // 3. Previous attempts (if retrying)
        let mut context_parts = vec![];

        // Load context tree docs if task has context_ref
        if let Some(ref ctx_ref) = task.context_ref {
            let ctx_docs = self.context_tree.get_docs(ctx_ref);
            if !ctx_docs.is_empty() {
                let mut tree_context = String::from("# Reference Documents\n\n");
                for doc_path in ctx_docs {
                    if let Ok(content) = std::fs::read_to_string(&doc_path) {
                        tree_context.push_str(&format!("## {}\n\n{}\n\n",
                            doc_path.display(), content));
                    }
                }
                context_parts.push(tree_context);
            }
        }

        // Add dependency results context
        let dep_context = self.docs.assemble_context(task_id, &dep_ids)?;
        if !dep_context.is_empty() {
            context_parts.push(dep_context);
        }

        // Include previous attempts context if retrying
        if let Some(attempts_ctx) = task.previous_attempts_context() {
            context_parts.push(attempts_ctx);
        }

        let context = context_parts.join("\n\n---\n\n");

        // Build prompt with role-specific instructions
        let prompt = self.build_task_prompt(&task, &context);

        let options = CliOptions {
            model: self.config.default_model,
            print: true,
            system_prompt: Some(context.clone()),
            ..Default::default()
        };

        let output = self.cli.run(&prompt, options).await
            .context("Failed to execute task")?;

        let success = output.exit_code == 0;

        // Save result with attempt number
        let result_id = if attempt_num > 1 {
            format!("{}-result-{}", task_id, attempt_num)
        } else {
            format!("{}-result", task_id)
        };

        let result_doc = Document::new(
            &result_id,
            DocType::TaskResult,
            format!("# Result: {} (Attempt #{})\n\n{}", task.subject, attempt_num, output.response),
        ).with_task_id(task_id);

        let result_path = self.docs.create(&result_doc)?;

        // Update task status
        if success {
            self.dag.get_task_mut(task_id).unwrap().complete(Some(result_path.clone()));

            // If this is a context fill task, add result to the target context node
            if let Some(target_node) = task.target_context_node() {
                if let Some(node) = self.context_tree.get_mut(target_node) {
                    node.add_doc(result_path.clone());
                    tracing::info!("Added result to context node: {}", target_node);
                }
            }

            // Parse and save any [ADD_CONTEXT:node_id] blocks from output
            let context_additions = self.parse_context_additions(&output.response);
            for (node_id, content) in context_additions {
                let add_doc = Document::new(
                    format!("{}-ctx-add-{}", task_id, node_id),
                    DocType::Context,
                    content,
                ).with_task_id(task_id);

                let add_path = self.docs.create(&add_doc)?;

                if let Some(node) = self.context_tree.get_mut(&node_id) {
                    node.add_doc(add_path.clone());
                    tracing::info!("Added discovered context to node {}: {:?}", node_id, add_path);
                } else {
                    tracing::warn!("Context node {} not found, skipping addition", node_id);
                }
            }
        } else {
            self.dag.get_task_mut(task_id).unwrap().fail("CLI execution failed");
        }

        // Save DAG and context tree state
        self.save_dag()?;
        self.save_context_tree()?;

        Ok(TaskResult {
            task_id: task_id.to_string(),
            success,
            output: output.response,
            session_id: output.session_id,
        })
    }

    /// Record a failed attempt and prepare for retry
    pub fn record_failed_attempt(
        &mut self,
        task_id: &str,
        output: &str,
        validation: &ValidationResult,
    ) -> Result<()> {
        let feedback = validation.issues.join("\n- ");
        let severity = match validation.severity {
            IssueSeverity::None => "none",
            IssueSeverity::Minor => "minor",
            IssueSeverity::Major => "major",
        };

        let attempt_count = {
            let task = self.dag.get_task_mut(task_id)
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

            task.record_attempt(output.to_string(), feedback, severity.to_string());
            task.reset_for_retry();
            task.attempts.len()
        };

        self.save_dag()?;

        tracing::info!(
            "Recorded attempt #{} for task {} (severity: {})",
            attempt_count,
            task_id,
            severity
        );

        Ok(())
    }

    /// Retry a failed task with accumulated context
    pub async fn retry_task(&mut self, task_id: &str) -> Result<TaskResult> {
        let task = self.dag.get_task(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

        if task.attempts.len() >= self.config.max_retries as usize {
            return Err(anyhow::anyhow!(
                "Task {} exceeded max retries ({})",
                task_id,
                self.config.max_retries
            ));
        }

        self.execute_task(task_id).await
    }

    /// Execute all tasks in dependency order
    pub async fn execute_all(&mut self) -> Result<Vec<TaskResult>> {
        let mut results = vec![];

        loop {
            let ready: Vec<String> = self.dag.ready_tasks()
                .iter()
                .map(|t| t.id.clone())
                .collect();

            if ready.is_empty() {
                if self.dag.is_complete() {
                    break;
                } else {
                    return Err(anyhow::anyhow!("No ready tasks but DAG not complete - possible deadlock"));
                }
            }

            // Execute ready tasks sequentially (for MVP)
            for task_id in ready {
                let result = self.execute_task(&task_id).await?;
                results.push(result);
            }
        }

        let stats = self.dag.stats();
        tracing::info!(
            "Execution complete: {}/{} succeeded, {} failed",
            stats.completed, stats.total, stats.failed
        );

        Ok(results)
    }

    /// Execute from an ExecutionPlan
    pub async fn execute_plan(&mut self, plan: &ExecutionPlan) -> Result<Vec<TaskResult>> {
        // Build DAG from plan
        self.dag = plan.build_dag()?;

        // Initialize context tree
        self.context_tree = plan.init_context_tree();

        // Save initial state
        self.save_dag()?;
        self.save_context_tree()?;

        // Execute all tasks
        self.execute_all().await
    }

    /// Get fork points from current DAG
    pub fn fork_points(&self) -> Vec<&Task> {
        self.dag.fork_points()
    }

    /// Get parallel branches from a fork point
    pub fn parallel_branches(&self, task_id: &str) -> Vec<&str> {
        self.dag.parallel_branches(task_id)
    }

    /// Validate a task result using Opus
    pub async fn validate(&self, task_id: &str) -> Result<ValidationResult> {
        let result_doc = self.docs.read_latest_result(task_id)?;

        let prompt = format!(
            r#"Validate this task result:

{}

Check for:
1. Completeness: Does it address all requirements?
2. Accuracy: Are the claims supported?
3. Consistency: Any contradictions?
4. Quality: Is it well-structured and clear?

Classify severity:
- "none": All good, no issues
- "minor": Formatting, docs, style, typos only
- "major": Missing functionality, broken logic, security issues

Respond with JSON:
{{"approved": true/false, "severity": "none|minor|major", "issues": ["issue1", ...], "suggestions": ["suggestion1", ...]}}"#,
            result_doc.content
        );

        let output = self.cli.validate(&prompt).await?;

        // Parse validation result (extract JSON from response)
        let validation: ValidationResult = self.parse_validation_json(&output.response)
            .unwrap_or(ValidationResult {
                approved: false,
                severity: IssueSeverity::Major,
                issues: vec!["Failed to parse validation response".to_string()],
                suggestions: vec![],
            });

        Ok(validation)
    }

    /// Multi-pass validation: run N checks in parallel, require threshold to pass
    pub async fn validate_multi(&self, task_id: &str) -> Result<MultiValidationResult> {
        self.validate_multi_with_threshold(
            task_id,
            self.config.validation_checks,
            self.config.validation_threshold,
        ).await
    }

    /// Multi-pass validation with custom threshold (for recheck after fix)
    pub async fn validate_multi_with_threshold(
        &self,
        task_id: &str,
        num_checks: u32,
        threshold: u32,
    ) -> Result<MultiValidationResult> {
        use futures::future::join_all;

        tracing::info!(
            "Running {}-pass validation for task {} (threshold: {})",
            num_checks, task_id, threshold
        );

        // Spawn validation tasks
        let result_doc = self.docs.read_latest_result(task_id)?;
        let content = result_doc.content.clone();

        let mut handles = vec![];
        for i in 1..=num_checks {
            let cli = self.cli.clone();
            let content = content.clone();

            let handle = tokio::spawn(async move {
                let prompt = format!(
                    r#"[Validation Check #{}/{}]

Validate this task result:

{}

Check for:
1. Completeness: Does it address all requirements?
2. Accuracy: Are the claims supported?
3. Consistency: Any contradictions?
4. Quality: Is it well-structured and clear?

Classify severity:
- "none": All good, no issues
- "minor": Formatting, docs, style, typos only
- "major": Missing functionality, broken logic, security issues

Respond with JSON:
{{"approved": true/false, "severity": "none|minor|major", "issues": ["issue1", ...], "suggestions": ["suggestion1", ...]}}"#,
                    i, num_checks, content
                );

                let output = cli.validate(&prompt).await;
                (i, output)
            });

            handles.push(handle);
        }

        // Collect results
        let results = join_all(handles).await;

        let mut checks = vec![];
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut worst_severity = IssueSeverity::None;

        for result in results {
            let (check_num, output_result) = result.context("Validation task panicked")?;

            let validation = match output_result {
                Ok(output) => {
                    self.parse_validation_json(&output.response)
                        .unwrap_or(ValidationResult {
                            approved: false,
                            severity: IssueSeverity::Major,
                            issues: vec!["Failed to parse validation response".to_string()],
                            suggestions: vec![],
                        })
                }
                Err(e) => {
                    tracing::warn!("Validation check #{} failed: {}", check_num, e);
                    ValidationResult {
                        approved: false,
                        severity: IssueSeverity::Major,
                        issues: vec![format!("Validation error: {}", e)],
                        suggestions: vec![],
                    }
                }
            };

            if validation.approved {
                passed += 1;
            } else {
                failed += 1;
            }

            // Track worst severity
            match (&worst_severity, &validation.severity) {
                (IssueSeverity::None, s) => worst_severity = *s,
                (IssueSeverity::Minor, IssueSeverity::Major) => worst_severity = IssueSeverity::Major,
                _ => {}
            }

            checks.push(validation);
        }

        let approved = passed >= threshold;

        tracing::info!(
            "Validation complete: {}/{} passed (threshold: {}) -> {}",
            passed, num_checks, threshold,
            if approved { "APPROVED" } else { "REJECTED" }
        );

        Ok(MultiValidationResult {
            passed,
            failed,
            total: num_checks,
            threshold,
            approved,
            severity: worst_severity,
            checks,
        })
    }

    /// Recheck after fix with potentially lower threshold
    pub async fn recheck(&self, task_id: &str) -> Result<MultiValidationResult> {
        self.validate_multi_with_threshold(
            task_id,
            self.config.validation_checks,
            self.config.recheck_threshold,
        ).await
    }

    /// Get DAG statistics
    pub fn stats(&self) -> DagStats {
        self.dag.stats()
    }

    /// Get all tasks
    pub fn tasks(&self) -> Vec<&Task> {
        self.dag.tasks().collect()
    }

    /// Parse [ADD_CONTEXT:node_id]...[/ADD_CONTEXT] blocks from output
    /// Returns Vec<(node_id, content)>
    fn parse_context_additions(&self, output: &str) -> Vec<(String, String)> {
        let mut additions = vec![];
        let mut remaining = output;

        while let Some(start_idx) = remaining.find("[ADD_CONTEXT:") {
            let after_tag = &remaining[start_idx + 13..]; // skip "[ADD_CONTEXT:"

            // Find the closing bracket of the opening tag
            if let Some(bracket_idx) = after_tag.find(']') {
                let node_id = after_tag[..bracket_idx].trim().to_string();
                let after_bracket = &after_tag[bracket_idx + 1..];

                // Find the closing tag
                if let Some(end_idx) = after_bracket.find("[/ADD_CONTEXT]") {
                    let content = after_bracket[..end_idx].trim().to_string();

                    if !node_id.is_empty() && !content.is_empty() {
                        additions.push((node_id, content));
                    }

                    remaining = &after_bracket[end_idx + 14..]; // skip "[/ADD_CONTEXT]"
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        additions
    }

    /// Build task prompt with role-specific instructions
    fn build_task_prompt(&self, task: &Task, context: &str) -> String {
        use crate::dag::TaskType;

        let role_instruction = match &task.task_type {
            TaskType::ContextFill { target_node } => format!(
r#"# Role: Context Preparer

You are preparing reference documents for context node '{}'.
Your output will be used by downstream worker tasks.

## Guidelines
- Research and gather relevant information for this context
- Output structured, reusable documentation
- Focus on facts and references that workers will need
- Be thorough - workers will rely on what you provide

## Output Format
Produce a well-organized document with clear sections.
"#,
                target_node
            ),

            TaskType::Worker => String::from(
r#"# Role: Implementation Worker

Reference documents are provided above. Use them as your primary source.

## Guidelines
- Use the provided context for your work
- Focus on implementation based on available information
- If context is insufficient for a specific part:
  - Do NOT guess or make assumptions
  - Spawn a research sub-agent to gather what you need
- Keep your work focused on the task at hand

## Sub-Agent Usage
When you need additional information not in the context:
```
[SPAWN:research] Query: <what you need to know>
```

## Adding Discovered Context
If you discover important information that should be shared with other tasks,
add it to the context tree using this format:

```
[ADD_CONTEXT:ctx-node-id]
## Title
Content that should be available to sibling/downstream tasks...
[/ADD_CONTEXT]
```

This will be saved and made available to other tasks referencing that context node.
"#
            ),
        };

        format!(
            "{}\n\n---\n\n{}\n\n---\n\n# Task: {}\n\n{}",
            role_instruction,
            context,
            task.subject,
            task.description
        )
    }

    fn parse_task_json(&self, response: &str) -> Result<Vec<PlannedTask>> {
        // Try to extract JSON array from response
        let json_str = if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        serde_json::from_str(json_str)
            .context("Failed to parse task JSON from response")
    }

    fn parse_validation_json(&self, response: &str) -> Result<ValidationResult> {
        // Try to extract JSON object from response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        serde_json::from_str(json_str)
            .context("Failed to parse validation JSON from response")
    }
}

#[derive(Debug, Deserialize)]
struct PlannedTask {
    id: String,
    subject: String,
    description: String,
    #[serde(default)]
    depends_on: Vec<String>,
}

/// Issue severity classification for targeted fixing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    #[default]
    None,   // All good
    Minor,  // Formatting, docs, style, typos
    Major,  // Missing functionality, broken logic, security
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub approved: bool,
    pub severity: IssueSeverity,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Multi-pass validation result aggregating multiple checks
#[derive(Debug, Clone)]
pub struct MultiValidationResult {
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
    pub threshold: u32,
    pub approved: bool,
    pub severity: IssueSeverity,
    pub checks: Vec<ValidationResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_orchestrator() -> (Orchestrator, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(data_dir.join("tasks")).unwrap();
        std::fs::create_dir_all(data_dir.join("results")).unwrap();
        std::fs::create_dir_all(data_dir.join("contexts")).unwrap();

        let orch = Orchestrator::new(temp_dir.path().to_path_buf(), data_dir).unwrap();
        (orch, temp_dir)
    }

    #[test]
    fn test_parse_context_additions_single() {
        let (orch, _temp) = create_test_orchestrator();

        let output = r#"
Task completed successfully.

[ADD_CONTEXT:ctx-branch-a]
## API Documentation
The API uses Bearer tokens for auth.
Endpoint: /api/v1/users
[/ADD_CONTEXT]

Done.
"#;

        let additions = orch.parse_context_additions(output);
        assert_eq!(additions.len(), 1);
        assert_eq!(additions[0].0, "ctx-branch-a");
        assert!(additions[0].1.contains("API Documentation"));
        assert!(additions[0].1.contains("Bearer tokens"));
    }

    #[test]
    fn test_parse_context_additions_multiple() {
        let (orch, _temp) = create_test_orchestrator();

        let output = r#"
[ADD_CONTEXT:ctx-auth]
## Auth Info
OAuth 2.0 flow
[/ADD_CONTEXT]

Some other content...

[ADD_CONTEXT:ctx-db]
## Database Schema
Users table has id, name, email
[/ADD_CONTEXT]
"#;

        let additions = orch.parse_context_additions(output);
        assert_eq!(additions.len(), 2);
        assert_eq!(additions[0].0, "ctx-auth");
        assert!(additions[0].1.contains("OAuth 2.0"));
        assert_eq!(additions[1].0, "ctx-db");
        assert!(additions[1].1.contains("Users table"));
    }

    #[test]
    fn test_parse_context_additions_none() {
        let (orch, _temp) = create_test_orchestrator();

        let output = "Just regular output with no context additions.";
        let additions = orch.parse_context_additions(output);
        assert!(additions.is_empty());
    }

    #[test]
    fn test_parse_context_additions_malformed() {
        let (orch, _temp) = create_test_orchestrator();

        // Missing closing tag
        let output = "[ADD_CONTEXT:ctx-test] Some content without closing tag";
        let additions = orch.parse_context_additions(output);
        assert!(additions.is_empty());

        // Missing node id
        let output2 = "[ADD_CONTEXT:] Content [/ADD_CONTEXT]";
        let additions2 = orch.parse_context_additions(output2);
        assert!(additions2.is_empty());
    }
}
