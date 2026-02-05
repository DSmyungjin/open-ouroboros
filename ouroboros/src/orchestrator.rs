use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

use crate::cli::{CliRunner, CliOptions, Model};
use crate::dag::{DagManager, DagStats, Task, ContextTree, ExecutionPlan};
use crate::docs::{DocumentStore, Document, DocType};
use crate::search::SearchEngine;
use crate::work_session::{WorkSessionManager, WorkSession};

pub struct Orchestrator {
    cli: CliRunner,
    dag: DagManager,
    docs: DocumentStore,
    search: Option<SearchEngine>,
    config: OrchestratorConfig,
    #[allow(dead_code)]
    base_data_dir: PathBuf,
    session_data_dir: PathBuf,
    context_tree: ContextTree,
    session_mgr: WorkSessionManager,
    current_session: Option<WorkSession>,
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
    /// Enable auto-search injection before task execution
    pub auto_search_enabled: bool,
    /// Maximum number of auto-search results to inject
    pub auto_search_max_results: usize,
    /// Minimum score threshold for auto-search results
    pub auto_search_min_score: f32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            default_model: Model::Sonnet,
            validation_model: Model::Sonnet,  // Opus → Sonnet (faster)
            extraction_model: Model::Haiku,
            auto_validate: false,
            max_retries: 3,
            validation_checks: 1,             // 3 → 1 (single check)
            validation_threshold: 1,          // 1 out of 1
            recheck_threshold: 1,
            auto_search_enabled: true,        // Enable by default
            auto_search_max_results: 5,       // Top 5 results
            auto_search_min_score: 0.3,       // Minimum relevance
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
        let session_mgr = WorkSessionManager::new(&data_dir)?;

        // Determine session data directory (current session or base)
        let session_data_dir = session_mgr.get_session_data_dir(None)
            .unwrap_or_else(|_| data_dir.clone());

        let docs = DocumentStore::new(&session_data_dir)?;

        // Try to load existing DAG from session
        let dag_path = session_data_dir.join("dag.json");
        let dag = if DagManager::exists(&dag_path) {
            DagManager::load(&dag_path).unwrap_or_else(|_| DagManager::new())
        } else {
            DagManager::new()
        };

        // Try to load existing context tree from session
        let ctx_path = session_data_dir.join("context-tree.json");
        let context_tree = if ctx_path.exists() {
            let content = std::fs::read_to_string(&ctx_path).ok();
            content
                .and_then(|c| serde_json::from_str(&c).ok())
                .map(ContextTree::from_state)
                .unwrap_or_else(ContextTree::new)
        } else {
            ContextTree::new()
        };

        // Load current session if exists
        let current_session = session_mgr.current_session().ok().flatten();

        // Initialize search engine (keyword-only mode)
        let search_path = session_data_dir.join("search_index");
        let search = SearchEngine::keyword_only(&search_path).ok();

        Ok(Self {
            cli,
            dag,
            docs,
            search,
            config: OrchestratorConfig::default(),
            base_data_dir: data_dir,
            session_data_dir,
            context_tree,
            session_mgr,
            current_session,
        })
    }

    pub fn with_config(mut self, config: OrchestratorConfig) -> Self {
        self.config = config;
        self
    }

    /// Save DAG state to file
    pub fn save_dag(&self) -> Result<()> {
        let dag_path = self.session_data_dir.join("dag.json");
        self.dag.save(&dag_path)
    }

    /// Save context tree to file
    pub fn save_context_tree(&self) -> Result<()> {
        let ctx_path = self.session_data_dir.join("context-tree.json");
        let state = self.context_tree.to_state();
        let content = serde_json::to_string_pretty(&state)?;
        std::fs::write(&ctx_path, content)?;
        Ok(())
    }

    /// Get current session info
    pub fn current_session(&self) -> Option<&WorkSession> {
        self.current_session.as_ref()
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<crate::work_session::WorkSessionSummary>> {
        self.session_mgr.list_sessions()
    }

    /// Switch to a different session
    pub fn switch_session(&mut self, session_id: &str) -> Result<()> {
        let session = self.session_mgr.switch_session(session_id)?;
        self.session_data_dir = self.session_mgr.session_dir(&session.id);
        self.docs = DocumentStore::new(&self.session_data_dir)?;

        // Reload DAG and context tree
        let dag_path = self.session_data_dir.join("dag.json");
        self.dag = if DagManager::exists(&dag_path) {
            DagManager::load(&dag_path).unwrap_or_else(|_| DagManager::new())
        } else {
            DagManager::new()
        };

        let ctx_path = self.session_data_dir.join("context-tree.json");
        self.context_tree = if ctx_path.exists() {
            let content = std::fs::read_to_string(&ctx_path).ok();
            content
                .and_then(|c| serde_json::from_str(&c).ok())
                .map(ContextTree::from_state)
                .unwrap_or_else(ContextTree::new)
        } else {
            ContextTree::new()
        };

        self.current_session = Some(session);
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

    /// Plan: Generate task DAG from a goal with optional label
    pub async fn plan_with_label(&mut self, goal: &str, label: Option<String>) -> Result<Vec<String>> {
        // Create a new work session
        let session = self.session_mgr.create_session(goal, label)?;
        let session_id = session.id.clone();

        tracing::info!("Created work session: {} for goal: {}", session_id, goal);

        // Update paths to use new session directory
        self.session_data_dir = self.session_mgr.session_dir(&session_id);
        self.docs = DocumentStore::new(&self.session_data_dir)?;
        self.dag = DagManager::new();
        self.context_tree = ContextTree::new();
        self.current_session = Some(session);

        // Reinitialize search engine for new session directory
        let search_path = self.session_data_dir.join("search_index");
        self.search = SearchEngine::keyword_only(&search_path).ok();

        // Generate plan ID for session tracking
        let plan_id = format!("plan-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());

        let prompt = format!(
            r#"[PLAN:{}] Task Planning

You are a task planner. Analyze the goal and create the MINIMUM number of tasks needed.

Goal: {}

## CRITICAL RULES

1. **DO NOT over-split**: Simple goals should be 1 task. Only split when truly necessary.
   - "Add ping() method" → 1 task (just implement it)
   - "Build entire API layer" → multiple tasks (genuinely complex)

2. **Skip unnecessary steps**:
   - Don't create "research" tasks for trivial things
   - Don't create "design" tasks for simple additions
   - Don't create separate "test" tasks unless explicitly requested

3. Use "context_fill" tasks ONLY when:
   - External research is genuinely needed (new library, unfamiliar domain)
   - Information must be shared across multiple workers

4. Most goals need just 1-2 "worker" tasks.

## Output Format (JSON)

{{
  "context_nodes": [
    {{"id": "ctx-root", "parent": null}}
  ],
  "tasks": [
    {{
      "id": "task-001",
      "subject": "Short description",
      "description": "Detailed instructions",
      "task_type": "worker",
      "target_node": null,
      "context_ref": "ctx-root",
      "depends_on": []
    }}
  ]
}}

Output ONLY the JSON object, no other text."#,
            plan_id, goal
        );

        let options = CliOptions {
            model: self.config.default_model,
            print: true,
            skip_permissions: true,
            system_prompt: Some("You are a task planner that outputs ONLY valid JSON. No explanations, no markdown formatting, just raw JSON.".to_string()),
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
        let plan: PlannedWorkflow = self.parse_plan_json(&output.response)?;

        // Build context tree
        self.context_tree = ContextTree::new();
        let root = self.context_tree.init_root();
        let root_id = root.node_id.clone();

        // Build a map from plan node IDs to actual node IDs
        // "ctx-root" in plan -> actual root_id
        let mut node_id_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        node_id_map.insert("ctx-root".to_string(), root_id.clone());

        // Create context nodes
        for node in &plan.context_nodes {
            if node.id == "ctx-root" {
                continue; // Already created
            }

            // Resolve parent ID using the map
            let parent_plan_id = node.parent.as_deref().unwrap_or("ctx-root");
            let parent_actual_id = node_id_map.get(parent_plan_id)
                .cloned()
                .unwrap_or_else(|| root_id.clone());

            if self.context_tree.get(&parent_actual_id).is_some() {
                // Strip "ctx-" prefix if present (branch_with_ids adds it)
                let node_base_id = node.id.strip_prefix("ctx-").unwrap_or(&node.id);

                // Create the node
                let _ = self.context_tree.branch_with_ids(
                    &parent_actual_id,
                    "plan",
                    &[node_base_id],
                    None,
                );

                // Map plan node ID to actual node ID
                let actual_node_id = format!("ctx-{}", node_base_id);
                node_id_map.insert(node.id.clone(), actual_node_id);
            }
        }

        // Add tasks to DAG
        let mut task_ids = vec![];
        for planned in &plan.tasks {
            let task = match planned.task_type.as_str() {
                "context_fill" => {
                    let plan_target = planned.target_node.as_deref().unwrap_or("ctx-root");
                    // Resolve to actual node ID
                    let actual_target = node_id_map.get(plan_target)
                        .cloned()
                        .unwrap_or_else(|| plan_target.to_string());
                    Task::new_context_fill(&planned.subject, &planned.description, &actual_target)
                        .with_id(&planned.id)
                }
                _ => {
                    let mut t = Task::new(&planned.subject, &planned.description)
                        .with_id(&planned.id);
                    if let Some(ref ctx_ref) = planned.context_ref {
                        // Resolve to actual node ID
                        let actual_ref = node_id_map.get(ctx_ref)
                            .cloned()
                            .unwrap_or_else(|| ctx_ref.clone());
                        t = t.with_context_ref(&actual_ref);
                    }
                    t
                }
            };

            self.dag.add_task(task)?;
            task_ids.push(planned.id.clone());

            // Save task definition
            let doc = Document::new(
                &planned.id,
                DocType::TaskDefinition,
                format!("# {}\n\n{}", planned.subject, planned.description),
            ).with_task_id(&planned.id);
            self.docs.create(&doc)?;

            // Index task for search
            if let Some(ref mut search) = self.search {
                let session_id = self.current_session.as_ref().map(|s| s.id.as_str());
                if let Err(e) = search.index_task(
                    &planned.id,
                    &planned.subject,
                    &planned.description,
                    session_id,
                ).await {
                    tracing::warn!("Failed to index task: {}", e);
                }
            }
        }

        // Add dependencies
        for planned in &plan.tasks {
            for dep_id in &planned.depends_on {
                self.dag.add_dependency(&planned.id, dep_id)?;
            }
        }

        // Save DAG and context tree to file
        self.save_dag()?;
        self.save_context_tree()?;

        // Update session with task count
        if let Some(ref mut session) = self.current_session {
            session.start(task_ids.len());
            self.session_mgr.update_session_in_index(session)?;
        }

        tracing::info!("Created {} tasks with {} context nodes in session {}",
            task_ids.len(), plan.context_nodes.len(),
            self.current_session.as_ref().map(|s| s.id.as_str()).unwrap_or("none"));
        Ok(task_ids)
    }

    /// Plan: Generate task DAG from a goal
    pub async fn plan(&mut self, goal: &str) -> Result<Vec<String>> {
        self.plan_with_label(goal, None).await
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

        // Auto-search for relevant knowledge (inject at the top)
        if self.config.auto_search_enabled {
            match self.auto_search_for_task(&task).await {
                Ok(auto_knowledge) if !auto_knowledge.is_empty() => {
                    tracing::info!("Injecting {} bytes of auto-discovered knowledge",
                        auto_knowledge.len());
                    // Insert at the beginning so it appears first
                    context_parts.insert(0, auto_knowledge);
                }
                Err(e) => {
                    tracing::warn!("Auto-search failed: {}", e);
                }
                _ => {}
            }
        }

        let context = context_parts.join("\n\n---\n\n");

        // Build prompt with role-specific instructions
        let prompt = self.build_task_prompt(&task, &context);

        let options = CliOptions {
            model: self.config.default_model,
            print: true,
            skip_permissions: true,
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

        // Index task result for search
        if let Some(ref mut search) = self.search {
            let session_id = self.current_session.as_ref().map(|s| s.id.as_str());
            if let Err(e) = search.index_task_result(task_id, &output.response, session_id).await {
                tracing::warn!("Failed to index task result: {}", e);
            }
        }

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
                let doc_id = format!("{}-ctx-add-{}", task_id, node_id);
                let add_doc = Document::new(
                    &doc_id,
                    DocType::Context,
                    content.clone(),
                ).with_task_id(task_id);

                let add_path = self.docs.create(&add_doc)?;

                // Index context for search
                if let Some(ref mut search) = self.search {
                    let session_id = self.current_session.as_ref().map(|s| s.id.as_str());
                    if let Err(e) = search.index_context(
                        &doc_id,
                        &format!("Context: {}", node_id),
                        &content,
                        session_id,
                        Some(task_id),
                    ).await {
                        tracing::warn!("Failed to index context: {}", e);
                    }
                }

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

        // Update session status
        if let Some(ref mut session) = self.current_session {
            let stats = self.dag.stats();
            session.completed_count = stats.completed;
            session.failed_count = stats.failed;
            if stats.completed + stats.failed >= stats.total && stats.total > 0 {
                session.completed_at = Some(chrono::Utc::now());
                if stats.failed == 0 {
                    session.status = crate::work_session::WorkSessionStatus::Completed;
                } else {
                    session.status = crate::work_session::WorkSessionStatus::Failed;
                }
            }
            self.session_mgr.update_session_in_index(session)?;
        }

        // Auto-validate and fix if enabled (only for successful worker tasks)
        let final_success = if success && self.config.auto_validate && !task.is_context_fill() {
            tracing::info!("Auto-validating task: {}", task_id);
            match self.auto_check_and_fix(task_id).await {
                Ok(passed) => {
                    if !passed {
                        // Update task status to reflect validation failure
                        self.dag.get_task_mut(task_id).unwrap().fail("Validation failed after fix attempt");
                        self.save_dag()?;
                    }
                    passed
                }
                Err(e) => {
                    tracing::warn!("Auto-validation error for {}: {}", task_id, e);
                    success // Keep original success on validation error
                }
            }
        } else {
            success
        };

        Ok(TaskResult {
            task_id: task_id.to_string(),
            success: final_success,
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

    /// Execute all tasks in dependency order (with parallel execution for independent tasks)
    pub async fn execute_all(&mut self) -> Result<Vec<TaskResult>> {
        use futures::future::join_all;

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

            if ready.len() == 1 {
                // Single task - execute directly
                let result = self.execute_task(&ready[0]).await?;
                results.push(result);
            } else {
                // Multiple ready tasks - execute in parallel
                tracing::info!("Executing {} tasks in parallel", ready.len());

                // Collect task info for parallel execution
                let tasks_info: Vec<_> = ready.iter()
                    .filter_map(|tid| {
                        self.dag.get_task(tid).map(|t| {
                            (tid.clone(), t.subject.clone(), t.description.clone(),
                             t.task_type.clone(), t.context_ref.clone(),
                             t.previous_attempts_context())
                        })
                    })
                    .collect();

                // Mark all as in progress
                for (task_id, _, _, _, _, _) in &tasks_info {
                    if let Some(task) = self.dag.get_task_mut(task_id) {
                        task.start();
                    }
                }

                // Get dependency results for context
                let dep_contexts: Vec<_> = ready.iter()
                    .map(|tid| {
                        let dep_ids: Vec<String> = self.dag.dependencies(tid)
                            .iter()
                            .map(|t| t.id.clone())
                            .collect();
                        self.docs.assemble_context(tid, &dep_ids).unwrap_or_default()
                    })
                    .collect();

                // Get context tree docs
                let ctx_tree_docs: Vec<Vec<std::path::PathBuf>> = tasks_info.iter()
                    .map(|(_, _, _, _, ctx_ref, _)| {
                        ctx_ref.as_ref()
                            .map(|r| self.context_tree.get_docs(r))
                            .unwrap_or_default()
                    })
                    .collect();

                // Spawn parallel tasks
                let cli = self.cli.clone();
                let model = self.config.default_model;

                let handles: Vec<_> = tasks_info.into_iter()
                    .zip(dep_contexts.into_iter())
                    .zip(ctx_tree_docs.into_iter())
                    .map(|(((task_id, subject, description, task_type, _, attempts_ctx), dep_ctx), tree_docs)| {
                        let cli = cli.clone();

                        tokio::spawn(async move {
                            // Build context
                            let mut context_parts = vec![];

                            // Context tree docs
                            if !tree_docs.is_empty() {
                                let mut tree_context = String::from("# Reference Documents\n\n");
                                for doc_path in tree_docs {
                                    if let Ok(content) = std::fs::read_to_string(&doc_path) {
                                        tree_context.push_str(&format!("## {}\n\n{}\n\n",
                                            doc_path.display(), content));
                                    }
                                }
                                context_parts.push(tree_context);
                            }

                            // Dependency context
                            if !dep_ctx.is_empty() {
                                context_parts.push(dep_ctx);
                            }

                            // Previous attempts
                            if let Some(attempts) = attempts_ctx {
                                context_parts.push(attempts);
                            }

                            let context = context_parts.join("\n\n---\n\n");

                            // Build role instruction
                            use crate::dag::TaskType;
                            let role_instruction = match &task_type {
                                TaskType::ContextFill { target_node } => format!(
                                    "# Role: Context Preparer\n\nYou are preparing reference documents for context node '{}'.\n",
                                    target_node
                                ),
                                TaskType::Worker => String::from(
                                    "# Role: Implementation Worker\n\nReference documents are provided above.\n"
                                ),
                            };

                            let prompt = format!(
                                "{}\n\n---\n\n{}\n\n---\n\n# Task: {}\n\n{}",
                                role_instruction, context, subject, description
                            );

                            let options = CliOptions {
                                model,
                                print: true,
                                skip_permissions: true,
                                system_prompt: Some(context),
                                ..Default::default()
                            };

                            let output = cli.run(&prompt, options).await;
                            (task_id, subject, task_type, output)
                        })
                    })
                    .collect();

                // Collect results
                let parallel_results = join_all(handles).await;

                for result in parallel_results {
                    let (task_id, subject, task_type, output_result) = result
                        .context("Task execution panicked")?;

                    match output_result {
                        Ok(output) => {
                            let success = output.exit_code == 0;

                            // Save result
                            let result_doc = Document::new(
                                format!("{}-result", task_id),
                                DocType::TaskResult,
                                format!("# Result: {}\n\n{}", subject, output.response),
                            ).with_task_id(&task_id);

                            let result_path = self.docs.create(&result_doc)?;

                            // Update task status
                            if let Some(task) = self.dag.get_task_mut(&task_id) {
                                if success {
                                    task.complete(Some(result_path.clone()));
                                } else {
                                    task.fail("Execution failed");
                                }
                            }

                            // Handle context fill task
                            if let crate::dag::TaskType::ContextFill { ref target_node } = task_type {
                                if let Some(node) = self.context_tree.get_mut(target_node) {
                                    node.add_doc(result_path.clone());
                                }
                            }

                            // Parse ADD_CONTEXT blocks
                            let additions = self.parse_context_additions(&output.response);
                            for (node_id, content) in additions {
                                let add_doc = Document::new(
                                    format!("{}-ctx-add-{}", task_id, node_id),
                                    DocType::Context,
                                    content,
                                ).with_task_id(&task_id);
                                if let Ok(add_path) = self.docs.create(&add_doc) {
                                    if let Some(node) = self.context_tree.get_mut(&node_id) {
                                        node.add_doc(add_path);
                                    }
                                }
                            }

                            results.push(TaskResult {
                                task_id,
                                success,
                                output: output.response,
                                session_id: output.session_id,
                            });
                        }
                        Err(e) => {
                            if let Some(task) = self.dag.get_task_mut(&task_id) {
                                task.fail(format!("Error: {}", e));
                            }
                            results.push(TaskResult {
                                task_id,
                                success: false,
                                output: format!("Error: {}", e),
                                session_id: None,
                            });
                        }
                    }
                }

                // Save state after parallel batch
                self.save_dag()?;
                self.save_context_tree()?;
            }
        }

        let stats = self.dag.stats();
        tracing::info!(
            "Execution complete: {}/{} succeeded, {} failed",
            stats.completed, stats.total, stats.failed
        );

        // Update session status
        if let Some(ref mut session) = self.current_session {
            session.completed_count = stats.completed;
            session.failed_count = stats.failed;
            if stats.completed + stats.failed >= stats.total {
                session.completed_at = Some(chrono::Utc::now());
                if stats.failed > 0 {
                    session.status = crate::work_session::WorkSessionStatus::Failed;
                } else {
                    session.status = crate::work_session::WorkSessionStatus::Completed;
                }
            }
            self.session_mgr.update_session_in_index(session)?;
        }

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
        let task = self.dag.get_task(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;
        let result_doc = self.docs.read_latest_result(task_id)?;

        // Get plan context (task description serves as plan)
        let plan = &task.description;

        // Get dependencies' context for additional understanding
        let dep_ids: Vec<String> = self.dag.dependencies(task_id)
            .iter()
            .map(|t| t.id.clone())
            .collect();
        let dependency_context = self.docs.assemble_context(task_id, &dep_ids)
            .unwrap_or_default();

        // Get context tree docs if available
        let context_docs = if let Some(ref ctx_ref) = task.context_ref {
            let docs = self.context_tree.get_docs(ctx_ref);
            if !docs.is_empty() {
                let mut ctx = String::new();
                for doc_path in docs {
                    if let Ok(content) = std::fs::read_to_string(&doc_path) {
                        ctx.push_str(&format!("### {}\n{}\n\n", doc_path.display(), content));
                    }
                }
                ctx
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let prompt = format!(
            r#"# Task that was supposed to be completed
{}

# Plan (what was supposed to be done)
{}

# Context (reference documents and dependencies)
{}
{}

# Execution Result (how it was done + outcome)
{}

Evaluate whether the CURRENT STATE fully meets ALL task requirements.

Guidelines:
- Focus on the END RESULT, not whether changes were made
- PASS if the current state satisfies ALL requirements completely
- FAIL if ANY requirement is not met or only partially addressed
- Check EVERY requirement mentioned in the task
- Verify the approach taken was appropriate for the task

Issue Classification (when FAIL):
- MINOR issues: formatting, style, comments, typos, cosmetic improvements
- MAJOR issues: missing functionality, broken logic, security issues, core requirements not met,
  BLOCKED by permissions or resources (e.g., "I need permission to write files")

At the end, provide your verdict:
1. If PASS: Write "VERDICT: PASS"
2. If FAIL with only MINOR issues: Write "VERDICT: FAIL_MINOR" then "MINOR_ISSUES:" with list
3. If FAIL with any MAJOR issues: Write "VERDICT: FAIL_MAJOR" then "MAJOR_ISSUES:" with list"#,
            task.subject,
            plan,
            context_docs,
            dependency_context,
            result_doc.content
        );

        let output = self.cli.validate(&prompt).await?;

        // Parse VERDICT-based response
        let validation = self.parse_verdict_response(&output.response);

        Ok(validation)
    }

    /// Parse VERDICT-based validation response
    fn parse_verdict_response(&self, response: &str) -> ValidationResult {
        let upper = response.to_uppercase();

        if upper.contains("VERDICT: PASS") {
            ValidationResult {
                approved: true,
                severity: IssueSeverity::None,
                issues: vec![],
                suggestions: vec![],
            }
        } else if upper.contains("VERDICT: FAIL_MINOR") {
            let minor_issues = self.extract_issues(response, "MINOR_ISSUES:");
            ValidationResult {
                approved: false,
                severity: IssueSeverity::Minor,
                issues: minor_issues,
                suggestions: vec![],
            }
        } else {
            // FAIL_MAJOR or unrecognized = treat as major
            let major_issues = self.extract_issues(response, "MAJOR_ISSUES:");
            ValidationResult {
                approved: false,
                severity: IssueSeverity::Major,
                issues: if major_issues.is_empty() {
                    vec!["Validation failed".to_string()]
                } else {
                    major_issues
                },
                suggestions: vec![],
            }
        }
    }

    /// Extract issues from response after a marker
    fn extract_issues(&self, response: &str, marker: &str) -> Vec<String> {
        let lower = response.to_lowercase();
        let marker_lower = marker.to_lowercase();

        if let Some(start) = lower.find(&marker_lower) {
            let rest = &response[start + marker.len()..];
            rest.lines()
                .map(|l| l.trim())
                .take_while(|l| !l.is_empty() || l.starts_with('-') || l.starts_with('*'))
                .filter(|l| l.starts_with('-') || l.starts_with('*'))
                .map(|l| l.trim_start_matches('-').trim_start_matches('*').trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        } else {
            vec![]
        }
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

        let task = self.dag.get_task(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
            .clone();
        let result_doc = self.docs.read_latest_result(task_id)?;
        let content = result_doc.content.clone();
        let subject = task.subject.clone();
        let description = task.description.clone();

        let mut handles = vec![];
        for i in 1..=num_checks {
            let cli = self.cli.clone();
            let content = content.clone();
            let subject = subject.clone();
            let description = description.clone();

            let handle = tokio::spawn(async move {
                let prompt = format!(
                    r#"[Validation Check #{}/{}]

Task: {}

{}

Execution result:
{}

Evaluate whether the result fully meets ALL task requirements.

Issue Classification (when FAIL):
- MINOR: formatting, style, comments, typos, cosmetic
- MAJOR: missing functionality, broken logic, security, BLOCKED by permissions

Verdict format:
1. PASS: "VERDICT: PASS"
2. FAIL_MINOR: "VERDICT: FAIL_MINOR" then "MINOR_ISSUES:" with list
3. FAIL_MAJOR: "VERDICT: FAIL_MAJOR" then "MAJOR_ISSUES:" with list"#,
                    i, num_checks, subject, description, content
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
                    self.parse_verdict_response(&output.response)
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

    /// Auto-check and fix: validate task result and run appropriate fixer if needed
    /// Returns true if task passed (either initially or after fix)
    pub async fn auto_check_and_fix(&mut self, task_id: &str) -> Result<bool> {
        // Step 1: Run multi-pass validation
        let validation = self.validate_multi(task_id).await?;

        if validation.approved {
            tracing::info!("Task {} passed validation", task_id);
            return Ok(true);
        }

        tracing::info!(
            "Task {} failed validation: {:?} issues",
            task_id, validation.severity
        );

        // Step 2: Run appropriate fixer based on severity
        let fix_result = match validation.severity {
            IssueSeverity::Major => {
                self.run_major_fixer(task_id, &validation).await?
            }
            IssueSeverity::Minor => {
                self.run_minor_fixer(task_id, &validation).await?
            }
            IssueSeverity::None => {
                // Shouldn't happen if not approved, but handle gracefully
                return Ok(true);
            }
        };

        // Step 3: Save fix result
        let fix_doc = Document::new(
            format!("{}-fix", task_id),
            DocType::Context,
            format!("# Fix Result\n\n## How\n{}\n\n## Result\n{}",
                fix_result.how, fix_result.result),
        ).with_task_id(task_id);
        self.docs.create(&fix_doc)?;

        // Step 4: Recheck with potentially lower threshold
        let recheck = self.recheck(task_id).await?;

        if recheck.approved {
            tracing::info!("Task {} passed recheck after fix", task_id);
            Ok(true)
        } else {
            tracing::warn!(
                "Task {} still failing after fix: {:?}",
                task_id, recheck.severity
            );
            // Record for potential retry
            let combined = ValidationResult {
                approved: false,
                severity: recheck.severity,
                issues: recheck.checks.iter()
                    .flat_map(|c| c.issues.clone())
                    .collect(),
                suggestions: vec![],
            };
            self.record_failed_attempt(task_id, &fix_result.result, &combined)?;
            Ok(false)
        }
    }

    /// Run MajorFixer: full authority to fix logic, functionality, etc.
    async fn run_major_fixer(
        &self,
        task_id: &str,
        validation: &MultiValidationResult,
    ) -> Result<FixerOutput> {
        let task = self.dag.get_task(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
            .clone();
        let result_doc = self.docs.read_latest_result(task_id)?;

        // Collect all issues from validation checks
        let major_issues: Vec<String> = validation.checks.iter()
            .filter(|c| c.severity == IssueSeverity::Major)
            .flat_map(|c| c.issues.clone())
            .collect();
        let minor_issues: Vec<String> = validation.checks.iter()
            .filter(|c| c.severity == IssueSeverity::Minor)
            .flat_map(|c| c.issues.clone())
            .collect();

        let minor_section = if minor_issues.is_empty() {
            String::new()
        } else {
            format!("\n\n## Minor issues (fix these too if possible):\n- {}",
                minor_issues.join("\n- "))
        };

        let prompt = format!(
            r#"# MajorFixer Task

## Original Task
{}

## Task Description
{}

## Current Result
{}

## Major Issues to Fix
- {}{}

## Your Authority
You have FULL AUTHORITY to:
- Modify logic and functionality
- Add missing features
- Fix bugs and security issues
- Make architectural adjustments
- Implement missing requirements
- Refactor code as needed

## Output Format
Provide your fix with these sections:

===HOW===
Explain what you changed and why.

===RESULT===
Summary of the fixes applied."#,
            task.subject,
            task.description,
            result_doc.content,
            major_issues.join("\n- "),
            minor_section
        );

        let options = CliOptions {
            model: Model::Opus,
            print: true,
            skip_permissions: true,
            ..Default::default()
        };

        let output = self.cli.run(&prompt, options).await?;
        self.parse_fixer_output(&output.response)
    }

    /// Run MinorFixer: cosmetic fixes only (formatting, style, docs)
    async fn run_minor_fixer(
        &self,
        task_id: &str,
        validation: &MultiValidationResult,
    ) -> Result<FixerOutput> {
        let task = self.dag.get_task(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
            .clone();
        let result_doc = self.docs.read_latest_result(task_id)?;

        let minor_issues: Vec<String> = validation.checks.iter()
            .filter(|c| c.severity == IssueSeverity::Minor)
            .flat_map(|c| c.issues.clone())
            .collect();

        let prompt = format!(
            r#"# MinorFixer Task

## Original Task
{}

## Task Description
{}

## Current Result
{}

## Minor Issues to Fix
- {}

## Your Authority (LIMITED)
You may ONLY fix:
- Formatting problems (indentation, spacing)
- Style inconsistencies
- Missing comments/documentation
- Typos
- Cosmetic improvements

You may NOT:
- Change any logic or functionality
- Add new features
- Fix bugs requiring logic changes
- Make architectural changes

## Output Format
Provide your fix with these sections:

===HOW===
Explain what cosmetic changes you made.

===RESULT===
Summary of the minor fixes applied."#,
            task.subject,
            task.description,
            result_doc.content,
            minor_issues.join("\n- ")
        );

        let options = CliOptions {
            model: Model::Sonnet,
            print: true,
            skip_permissions: true,
            ..Default::default()
        };

        let output = self.cli.run(&prompt, options).await?;
        self.parse_fixer_output(&output.response)
    }

    /// Parse fixer output into FixerOutput struct
    fn parse_fixer_output(&self, output: &str) -> Result<FixerOutput> {
        let how_marker = "===HOW===";
        let result_marker = "===RESULT===";

        let how_start = output.find(how_marker);
        let result_start = output.find(result_marker);

        let (how, result) = match (how_start, result_start) {
            (Some(h), Some(r)) if h < r => {
                let how = output[h + how_marker.len()..r].trim().to_string();
                let result = output[r + result_marker.len()..].trim().to_string();
                (how, result)
            }
            _ => {
                // Fallback: use full output as both
                (output.to_string(), output.to_string())
            }
        };

        Ok(FixerOutput { how, result })
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

    /// Extract keywords from task for auto-search
    fn extract_keywords(&self, subject: &str, description: &str) -> Vec<String> {
        // Combine subject and description
        let text = format!("{} {}", subject, description);

        // Simple keyword extraction: split by whitespace and punctuation
        // Filter out common stop words and short words
        let stop_words = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "must", "and", "or", "but", "if", "then",
            "else", "when", "where", "why", "how", "what", "which", "who", "whom",
            "this", "that", "these", "those", "it", "its", "for", "from", "to",
            "of", "in", "on", "at", "by", "with", "about", "into", "through",
            "를", "을", "이", "가", "은", "는", "에", "의", "로", "으로", "와", "과",
            "도", "만", "까지", "부터", "에서", "한다", "하는", "하고", "하여",
        ];

        let keywords: Vec<String> = text
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| s.len() >= 2 && !stop_words.contains(&s.as_str()))
            .collect::<std::collections::HashSet<_>>()  // Deduplicate
            .into_iter()
            .take(10)  // Limit to 10 keywords
            .collect();

        keywords
    }

    /// Auto-search for relevant knowledge before task execution
    /// Searches across ALL sessions for relevant knowledge
    async fn auto_search_for_task(&self, task: &crate::dag::Task) -> Result<String> {
        // Extract keywords from task
        let keywords = self.extract_keywords(&task.subject, &task.description);
        if keywords.is_empty() {
            return Ok(String::new());
        }

        // Build search query from keywords
        let query = keywords.join(" ");
        tracing::debug!("Auto-search query: {}", query);

        // Search across all sessions
        let mut all_results = Vec::new();

        // Get all session directories
        let sessions = self.session_mgr.list_sessions()?;
        for session_summary in &sessions {
            let session_dir = self.session_mgr.session_dir(&session_summary.id);
            let search_index_path = session_dir.join("search_index");

            if !search_index_path.exists() {
                continue;
            }

            // Create read-only search engine for this session
            match crate::search::SearchEngine::keyword_reader_only(&search_index_path) {
                Ok(session_search) => {
                    let options = crate::search::SearchOptions::new()
                        .with_limit(self.config.auto_search_max_results)
                        .with_min_score(self.config.auto_search_min_score);

                    match session_search.search(&query, &options).await {
                        Ok(results) => {
                            for mut result in results {
                                // Add session info to title for context
                                result.title = format!("[{}] {}",
                                    session_summary.id.chars().take(10).collect::<String>(),
                                    result.title);
                                all_results.push(result);
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Search failed for session {}: {}", session_summary.id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Failed to open search index for session {}: {}", session_summary.id, e);
                }
            }
        }

        // Sort by score and limit results
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let results: Vec<_> = all_results.into_iter()
            .take(self.config.auto_search_max_results)
            .collect();

        if results.is_empty() {
            tracing::debug!("Auto-search found no results");
            return Ok(String::new());
        }

        tracing::info!("Auto-search found {} relevant documents for task {}",
            results.len(), task.id);

        // Format results as context
        let mut output = String::from("## 📚 Auto-Discovered Knowledge\n\n");
        output.push_str("아래는 이 태스크와 관련된 기존 지식입니다. 참고하여 작업하세요.\n\n");
        output.push_str("---\n\n");

        for result in results {
            let doc_type = result.doc_type.as_str();
            let score = (result.score * 100.0) as u32;

            output.push_str(&format!("### [{}] {}\n", doc_type, result.title));

            // Truncate content if too long (max 500 chars)
            let content_preview = if result.content.chars().count() > 500 {
                format!("{}...", result.content.chars().take(500).collect::<String>())
            } else {
                result.content.clone()
            };

            output.push_str(&format!("> {}\n", content_preview.replace('\n', "\n> ")));
            output.push_str(&format!("> **관련도: {}%**\n\n", score));
            output.push_str("---\n\n");
        }

        output.push_str(&format!("검색어: \"{}\"\n\n", keywords.join("\", \"")));

        Ok(output)
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

## Searching Past Knowledge
Before researching externally, search the knowledge base for relevant information:
```bash
# Search all sessions for relevant knowledge
ouroboros search "keyword" --all

# Filter by knowledge entries
ouroboros search "keyword" -t knowledge --all
```

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
  - Search past knowledge or spawn a research sub-agent
- Keep your work focused on the task at hand

## Searching Past Knowledge
Before researching externally, search the knowledge base for relevant information:
```bash
# Search current session
ouroboros search "keyword"

# Search all sessions
ouroboros search "keyword" --all

# Filter by document type
ouroboros search "keyword" -t knowledge
ouroboros search "keyword" -t task_result
```

Use search when:
- You need reference to past implementations
- Looking for design decisions or patterns used before
- Finding related task results or knowledge entries

## Sub-Agent Usage
When you need NEW information not in context or search results:
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

    #[allow(dead_code)]
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

    fn parse_plan_json(&self, response: &str) -> Result<PlannedWorkflow> {
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

        // Try parsing as full workflow first
        if let Ok(workflow) = serde_json::from_str::<PlannedWorkflow>(json_str) {
            return Ok(workflow);
        }

        // Fallback: try parsing as simple task array (backward compatibility)
        if let Ok(tasks) = serde_json::from_str::<Vec<PlannedTask>>(json_str) {
            return Ok(PlannedWorkflow {
                context_nodes: vec![PlannedContextNode {
                    id: "ctx-root".to_string(),
                    parent: None,
                }],
                tasks,
            });
        }

        // Try extracting array if object parsing failed
        let array_str = if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                &response[start..=end]
            } else {
                json_str
            }
        } else {
            json_str
        };

        let tasks: Vec<PlannedTask> = serde_json::from_str(array_str)
            .context("Failed to parse plan JSON from response")?;

        Ok(PlannedWorkflow {
            context_nodes: vec![PlannedContextNode {
                id: "ctx-root".to_string(),
                parent: None,
            }],
            tasks,
        })
    }

}

/// Planned workflow with context tree and tasks
#[derive(Debug, Deserialize)]
struct PlannedWorkflow {
    #[serde(default)]
    context_nodes: Vec<PlannedContextNode>,
    tasks: Vec<PlannedTask>,
}

#[derive(Debug, Deserialize)]
struct PlannedContextNode {
    id: String,
    parent: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlannedTask {
    id: String,
    subject: String,
    description: String,
    #[serde(default = "default_task_type")]
    task_type: String,
    #[serde(default)]
    target_node: Option<String>,
    #[serde(default)]
    context_ref: Option<String>,
    #[serde(default)]
    depends_on: Vec<String>,
}

fn default_task_type() -> String {
    "worker".to_string()
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

/// Output from MajorFixer or MinorFixer
#[derive(Debug, Clone)]
pub struct FixerOutput {
    pub how: String,
    pub result: String,
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
