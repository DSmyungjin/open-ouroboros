use std::path::PathBuf;
use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use ouroboros::Orchestrator;
use ouroboros::orchestrator::OrchestratorConfig;

#[derive(Parser)]
#[command(name = "ouroboros")]
#[command(about = "LLM Agent Orchestration System", long_about = None)]
struct Cli {
    /// Working directory
    #[arg(short, long, default_value = ".")]
    workdir: PathBuf,

    /// Data directory
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize project structure
    Init,

    /// Plan tasks from a goal
    Plan {
        /// The goal to achieve
        goal: String,

        /// Optional label for the session (e.g., "api-refactor")
        #[arg(short, long)]
        label: Option<String>,
    },

    /// Manage work sessions
    WorkSessions {
        #[command(subcommand)]
        action: WorkSessionAction,
    },

    /// List all tasks
    Tasks,

    /// Run a specific task or all tasks
    Run {
        /// Task ID to run (omit for all)
        task_id: Option<String>,

        /// Run all tasks
        #[arg(long)]
        all: bool,

        /// Auto-validate and fix after each task
        #[arg(long)]
        auto_validate: bool,
    },

    /// Validate a task result (single check)
    Validate {
        /// Task ID to validate
        task_id: String,

        /// Run multi-pass validation (N checks in parallel)
        #[arg(short, long)]
        multi: bool,
    },

    /// Retry a failed task with accumulated context
    Retry {
        /// Task ID to retry
        task_id: String,
    },

    /// Validate and fix a task result
    Fix {
        /// Task ID to validate and fix
        task_id: String,
    },

    /// Show execution statistics
    Stats,

    /// List Claude Code sessions for this project
    Sessions {
        /// Show only root sessions (non-forks)
        #[arg(long)]
        roots: bool,

        /// Search by keyword or task tag
        #[arg(short, long)]
        search: Option<String>,
    },

    /// Search indexed documents
    Search {
        /// Search query
        query: String,

        /// Filter by document type (task, task_result, context, knowledge)
        #[arg(short = 't', long)]
        doc_type: Option<String>,

        /// Search in specific session (by ID or prefix)
        #[arg(short, long)]
        session: Option<String>,

        /// Search across all sessions
        #[arg(short, long)]
        all: bool,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Filter by start date (format: YYYY-MM-DD or YYYY-MM-DD HH:MM:SS)
        #[arg(long)]
        from: Option<String>,

        /// Filter by end date (format: YYYY-MM-DD or YYYY-MM-DD HH:MM:SS)
        #[arg(long)]
        to: Option<String>,
    },

    /// Start API server
    Server {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to bind to
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// JWT secret key (can also use JWT_SECRET env var)
        #[arg(long)]
        jwt_secret: Option<String>,
    },
}

#[derive(Subcommand)]
enum WorkSessionAction {
    /// List all work sessions
    List,

    /// Show current session info
    Current,

    /// Switch to a different session
    Switch {
        /// Session ID or prefix
        session_id: String,
    },

    /// Show details of a session
    Show {
        /// Session ID or prefix
        session_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "ouroboros=info".into())
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            init_project(&cli.data_dir)?;
            println!("Project initialized at {:?}", cli.data_dir);
        }

        Commands::Plan { ref goal, ref label } => {
            let mut orch = create_orchestrator(&cli)?;
            let task_ids = orch.plan_with_label(goal, label.clone()).await?;

            if let Some(session) = orch.current_session() {
                println!("Session: {} ({})", session.id, session.goal);
            }
            println!("Created {} tasks:", task_ids.len());
            for id in task_ids {
                println!("  - {}", id);
            }
        }

        Commands::WorkSessions { ref action } => {
            use ouroboros::work_session::WorkSessionStatus;

            let mut orch = create_orchestrator(&cli)?;

            match action {
                WorkSessionAction::List => {
                    let sessions = orch.list_sessions()?;
                    if sessions.is_empty() {
                        println!("No work sessions found. Run 'ouroboros plan \"<goal>\"' to create one.");
                    } else {
                        println!("Work Sessions:");
                        println!("{}", "=".repeat(70));
                        for s in sessions {
                            let status_icon = match s.status {
                                WorkSessionStatus::Pending => "○",
                                WorkSessionStatus::Running => "◐",
                                WorkSessionStatus::Completed => "●",
                                WorkSessionStatus::Failed => "✗",
                                WorkSessionStatus::Archived => "◌",
                            };
                            let label_str = s.label.as_ref()
                                .map(|l| format!(" [{}]", l))
                                .unwrap_or_default();
                            let progress = format!("{}/{}", s.completed_count, s.task_count);
                            let date = s.created_at.format("%Y-%m-%d %H:%M").to_string();
                            let goal_short: String = s.goal.chars().take(37).collect();
                            let goal_short = if s.goal.chars().count() > 40 {
                                format!("{}...", goal_short)
                            } else {
                                s.goal.clone()
                            };

                            // Detect format: new "001-abc123-..." vs legacy "abc123-..."
                            // New format has 3 digits + dash at start
                            let is_new_format = s.id.len() >= 4
                                && s.id.chars().take(3).all(|c| c.is_ascii_digit())
                                && s.id.chars().nth(3) == Some('-');
                            let short_id_len = if is_new_format { 10 } else { 6 };
                            let short_id = &s.id[..short_id_len.min(s.id.len())];
                            println!("{} {}{}  {}  {}  {}",
                                status_icon, short_id, label_str, progress, date, goal_short);
                        }
                    }
                }

                WorkSessionAction::Current => {
                    if let Some(session) = orch.current_session() {
                        println!("Current Session: {}", session.id);
                        println!("  Goal:     {}", session.goal);
                        println!("  Status:   {:?}", session.status);
                        println!("  Progress: {}/{}", session.completed_count, session.task_count);
                        println!("  Created:  {}", session.created_at.format("%Y-%m-%d %H:%M"));
                        if let Some(label) = &session.label {
                            println!("  Label:    {}", label);
                        }
                    } else {
                        println!("No current session. Run 'ouroboros plan \"<goal>\"' to create one.");
                    }
                }

                WorkSessionAction::Switch { ref session_id } => {
                    orch.switch_session(session_id)?;
                    if let Some(session) = orch.current_session() {
                        println!("Switched to session: {}", session.id);
                        println!("  Goal: {}", session.goal);
                    }
                }

                WorkSessionAction::Show { ref session_id } => {
                    orch.switch_session(session_id)?;
                    if let Some(session) = orch.current_session() {
                        println!("Session: {}", session.id);
                        println!("  Goal:      {}", session.goal);
                        println!("  Status:    {:?}", session.status);
                        println!("  Progress:  {}/{}", session.completed_count, session.task_count);
                        println!("  Failed:    {}", session.failed_count);
                        println!("  Created:   {}", session.created_at.format("%Y-%m-%d %H:%M"));
                        if let Some(completed) = session.completed_at {
                            println!("  Completed: {}", completed.format("%Y-%m-%d %H:%M"));
                        }
                        if let Some(label) = &session.label {
                            println!("  Label:     {}", label);
                        }

                        // Show tasks
                        println!("\nTasks:");
                        for task in orch.tasks() {
                            let status = match &task.status {
                                ouroboros::dag::TaskStatus::Pending => "[ ]",
                                ouroboros::dag::TaskStatus::InProgress => "[~]",
                                ouroboros::dag::TaskStatus::Completed => "[✓]",
                                ouroboros::dag::TaskStatus::Failed { .. } => "[✗]",
                            };
                            println!("  {} {} - {}", status, task.id, task.subject);
                        }
                    }
                }
            }
        }

        Commands::Tasks => {
            let orch = create_orchestrator(&cli)?;
            let tasks = orch.tasks();
            if tasks.is_empty() {
                println!("No tasks found. Run 'ouroboros plan \"<goal>\"' first.");
            } else {
                println!("Tasks:");
                for task in tasks {
                    let status = match &task.status {
                        ouroboros::dag::TaskStatus::Pending => "[ ]",
                        ouroboros::dag::TaskStatus::InProgress => "[~]",
                        ouroboros::dag::TaskStatus::Completed => "[✓]",
                        ouroboros::dag::TaskStatus::Failed { .. } => "[✗]",
                    };
                    println!("  {} {} - {}", status, task.id, task.subject);
                }
            }
        }

        Commands::Run { ref task_id, all, auto_validate } => {
            let mut orch = create_orchestrator_with_config(&cli, auto_validate)?;

            if auto_validate {
                println!("Auto-validation enabled: tasks will be checked and fixed if needed");
            }

            if all || task_id.is_none() {
                println!("Running all tasks...");
                let results = orch.execute_all().await?;
                println!("\nCompleted {} tasks:", results.len());
                for result in results {
                    let status = if result.success { "✓" } else { "✗" };
                    println!("  [{}] {}", status, result.task_id);
                }
            } else if let Some(ref id) = task_id {
                println!("Running task: {}", id);
                let result = orch.execute_task(id).await?;
                if result.success {
                    println!("Task completed successfully");
                } else {
                    println!("Task failed");
                }
            }
        }

        Commands::Validate { ref task_id, multi } => {
            use ouroboros::orchestrator::IssueSeverity;

            let orch = create_orchestrator(&cli)?;

            if multi {
                // Multi-pass validation
                println!("Running multi-pass validation for task: {}", task_id);
                let result = orch.validate_multi(task_id).await?;

                let severity_str = match result.severity {
                    IssueSeverity::None => "none",
                    IssueSeverity::Minor => "minor",
                    IssueSeverity::Major => "major",
                };

                println!("\nResults: {}/{} passed (threshold: {})",
                    result.passed, result.total, result.threshold);

                if result.approved {
                    println!("✓ Validation PASSED (worst severity: {})", severity_str);
                } else {
                    println!("✗ Validation FAILED (worst severity: {})", severity_str);
                }

                // Show details from each check
                for (i, check) in result.checks.iter().enumerate() {
                    let check_status = if check.approved { "✓" } else { "✗" };
                    let check_sev = match check.severity {
                        IssueSeverity::None => "none",
                        IssueSeverity::Minor => "minor",
                        IssueSeverity::Major => "major",
                    };
                    println!("\n  Check #{}: {} ({})", i + 1, check_status, check_sev);
                    if !check.issues.is_empty() {
                        for issue in &check.issues {
                            println!("    - {}", issue);
                        }
                    }
                }
            } else {
                // Single validation
                println!("Validating task: {}", task_id);
                let result = orch.validate(task_id).await?;

                let severity_str = match result.severity {
                    IssueSeverity::None => "none",
                    IssueSeverity::Minor => "minor",
                    IssueSeverity::Major => "major",
                };

                if result.approved {
                    println!("✓ Validation passed (severity: {})", severity_str);
                } else {
                    println!("✗ Validation failed (severity: {})", severity_str);
                    if !result.issues.is_empty() {
                        println!("Issues:");
                        for issue in &result.issues {
                            println!("  - {}", issue);
                        }
                    }
                }

                if !result.suggestions.is_empty() {
                    println!("Suggestions:");
                    for suggestion in &result.suggestions {
                        println!("  - {}", suggestion);
                    }
                }
            }
        }

        Commands::Retry { ref task_id } => {
            use ouroboros::orchestrator::IssueSeverity;

            let mut orch = create_orchestrator(&cli)?;

            // First validate the current result to get feedback
            println!("Validating task before retry: {}", task_id);
            let validation = orch.validate(task_id).await?;

            if validation.approved {
                println!("✓ Task already passes validation, no retry needed");
                return Ok(());
            }

            let severity_str = match validation.severity {
                IssueSeverity::None => "none",
                IssueSeverity::Minor => "minor",
                IssueSeverity::Major => "major",
            };

            // Get current output for context
            let current_output = orch.tasks()
                .iter()
                .find(|t| t.id == *task_id)
                .and_then(|t| t.result_doc.as_ref())
                .map(|p| std::fs::read_to_string(p).unwrap_or_default())
                .unwrap_or_default();

            // Record the failed attempt
            orch.record_failed_attempt(task_id, &current_output, &validation)?;

            println!("Recording attempt (severity: {})", severity_str);
            println!("Issues: {:?}", validation.issues);

            // Retry with accumulated context
            println!("\nRetrying task: {}", task_id);
            let result = orch.retry_task(task_id).await?;

            if result.success {
                println!("✓ Retry completed");
            } else {
                println!("✗ Retry failed");
            }
        }

        Commands::Fix { ref task_id } => {
            use ouroboros::orchestrator::IssueSeverity;

            let mut orch = create_orchestrator(&cli)?;

            println!("Running auto-check and fix for task: {}", task_id);
            let passed = orch.auto_check_and_fix(task_id).await?;

            if passed {
                println!("✓ Task passed (either initially or after fix)");
            } else {
                // Get final validation state
                let result = orch.validate_multi(task_id).await?;
                let severity_str = match result.severity {
                    IssueSeverity::None => "none",
                    IssueSeverity::Minor => "MINOR",
                    IssueSeverity::Major => "MAJOR",
                };
                println!("✗ Task still failing after fix attempt");
                println!("  Severity: {}", severity_str);
                println!("  Passed: {}/{} (threshold: {})", result.passed, result.total, result.threshold);
            }
        }

        Commands::Stats => {
            let orch = create_orchestrator(&cli)?;
            let stats = orch.stats();
            println!("Task Statistics:");
            println!("  Total:       {}", stats.total);
            println!("  Completed:   {}", stats.completed);
            println!("  Failed:      {}", stats.failed);
            println!("  Pending:     {}", stats.pending);
            println!("  In Progress: {}", stats.in_progress);
        }

        Commands::Sessions { roots, ref search } => {
            use ouroboros::SessionManager;

            let sm = SessionManager::new(&cli.workdir)?;

            if !sm.has_sessions() {
                println!("No Claude Code sessions found for this project.");
                println!("Run some tasks first to create sessions.");
                return Ok(());
            }

            let sessions = if let Some(query) = search {
                // Search by keyword or task tag
                if query.starts_with("task-") {
                    // Search for task tag
                    sm.find_for_task(query)?
                        .map(|s| vec![s])
                        .unwrap_or_default()
                } else {
                    // General keyword search (includes conversation)
                    sm.find_by_tag_full(query)?
                }
            } else if roots {
                sm.list_roots()?
            } else {
                sm.list_sessions()?
            };

            if sessions.is_empty() {
                if search.is_some() {
                    println!("No sessions found matching query.");
                } else {
                    println!("No sessions found.");
                }
                return Ok(());
            }

            println!("Claude Code Sessions{}:",
                search.as_ref().map(|s| format!(" (search: {})", s)).unwrap_or_default());
            println!("{}", "=".repeat(70));

            for (i, session) in sessions.iter().enumerate() {
                let fork_marker = if session.is_sidechain { " [FORK]" } else { "" };
                let summary = session.summary.as_deref().unwrap_or("(no summary)");
                let prompt = session.first_prompt.as_deref()
                    .map(|p| if p.len() > 60 { format!("{}...", &p[..60]) } else { p.to_string() })
                    .unwrap_or_default();

                println!("[{}] {}{}", i + 1, &session.session_id[..8], fork_marker);
                println!("    {}", summary);
                if !prompt.is_empty() {
                    println!("    prompt: {}", prompt);
                }
                println!("    {} msgs | {}", session.message_count, &session.modified[..10]);
                println!();
            }

            if search.is_none() {
                println!("Search: ouroboros sessions --search <keyword>");
                println!("Fork:   ouroboros fork --root <session_id> <tasks>");
            }
        }

        Commands::Search { ref query, ref doc_type, ref session, all, limit, ref from, ref to } => {
            use ouroboros::search::{SearchEngine, SearchOptions, DocumentType, SearchResult};
            use ouroboros::work_session::WorkSessionManager;

            let session_mgr = WorkSessionManager::new(&cli.data_dir)?;

            // Collect search index paths based on options
            let search_paths: Vec<(PathBuf, Option<String>)> = if all {
                // Search all sessions
                let sessions_dir = cli.data_dir.join("sessions");
                if sessions_dir.exists() {
                    std::fs::read_dir(&sessions_dir)?
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .map(|e| {
                            let session_id = e.file_name().to_string_lossy().to_string();
                            (e.path().join("search_index"), Some(session_id))
                        })
                        .filter(|(p, _)| p.exists())
                        .collect()
                } else {
                    vec![]
                }
            } else if let Some(ref sid) = session {
                // Search specific session by ID or prefix
                let sessions_dir = cli.data_dir.join("sessions");
                if sessions_dir.exists() {
                    let matching: Vec<_> = std::fs::read_dir(&sessions_dir)?
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            name.starts_with(sid) || name.contains(sid)
                        })
                        .collect();

                    if matching.is_empty() {
                        println!("No session found matching: {}", sid);
                        return Ok(());
                    } else if matching.len() > 1 {
                        println!("Multiple sessions match '{}'. Be more specific:", sid);
                        for m in &matching {
                            println!("  - {}", m.file_name().to_string_lossy());
                        }
                        return Ok(());
                    }

                    let session_id = matching[0].file_name().to_string_lossy().to_string();
                    let path = matching[0].path().join("search_index");
                    if path.exists() {
                        vec![(path, Some(session_id))]
                    } else {
                        println!("No search index found for session: {}", sid);
                        return Ok(());
                    }
                } else {
                    vec![]
                }
            } else {
                // Search current session only
                if let Ok(Some(current)) = session_mgr.current_session() {
                    let path = cli.data_dir.join("sessions").join(&current.id).join("search_index");
                    if path.exists() {
                        vec![(path, Some(current.id.clone()))]
                    } else {
                        vec![]
                    }
                } else {
                    let path = cli.data_dir.join("search_index");
                    if path.exists() {
                        vec![(path, None)]
                    } else {
                        vec![]
                    }
                }
            };

            if search_paths.is_empty() {
                println!("No search index found.");
                println!("Index will be created when documents are added.");
                return Ok(());
            }

            // Build search options
            let mut options = SearchOptions::new().with_limit(limit);

            if let Some(ref dtype) = doc_type {
                let dt = match dtype.as_str() {
                    "task" => DocumentType::Task,
                    "task_result" | "result" => DocumentType::TaskResult,
                    "context" => DocumentType::Context,
                    "knowledge" => DocumentType::Knowledge,
                    "plan" => DocumentType::Plan,
                    _ => {
                        println!("Unknown document type: {}. Use: task, task_result, context, knowledge, plan", dtype);
                        return Ok(());
                    }
                };
                options = options.with_doc_type(dt);
            }

            // Parse date filters
            if let Some(ref from_str) = from {
                match parse_date_string(from_str) {
                    Ok(date) => {
                        options = options.with_date_from(date);
                    }
                    Err(e) => {
                        println!("Invalid 'from' date format: {}. Use YYYY-MM-DD or YYYY-MM-DD HH:MM:SS", e);
                        return Ok(());
                    }
                }
            }

            if let Some(ref to_str) = to {
                match parse_date_string(to_str) {
                    Ok(date) => {
                        options = options.with_date_to(date);
                    }
                    Err(e) => {
                        println!("Invalid 'to' date format: {}. Use YYYY-MM-DD or YYYY-MM-DD HH:MM:SS", e);
                        return Ok(());
                    }
                }
            }

            // Search across all paths and collect results
            let mut all_results: Vec<(SearchResult, String)> = Vec::new();

            for (search_path, session_id) in &search_paths {
                let engine = SearchEngine::keyword_reader_only(search_path)?;
                let results = engine.search(query, &options).await?;
                let sid = session_id.clone().unwrap_or_else(|| "default".to_string());
                for r in results {
                    all_results.push((r, sid.clone()));
                }
            }

            // Sort by score descending
            all_results.sort_by(|a, b| b.0.score.partial_cmp(&a.0.score).unwrap_or(std::cmp::Ordering::Equal));

            // Limit total results
            all_results.truncate(limit);

            if all_results.is_empty() {
                println!("No results found for: \"{}\"", query);
                if all {
                    println!("Searched {} session(s)", search_paths.len());
                }
                return Ok(());
            }

            let session_info = if all {
                format!(" (across {} sessions)", search_paths.len())
            } else if let Some((_, ref sid)) = all_results.first() {
                format!(" [session: {}]", &sid[..8.min(sid.len())])
            } else {
                String::new()
            };

            println!("Search results for \"{}\"{} ({} found):", query, session_info, all_results.len());
            println!("{}", "=".repeat(70));

            for (i, (r, sid)) in all_results.iter().enumerate() {
                let content_preview: String = r.content
                    .chars()
                    .take(200)
                    .collect::<String>()
                    .replace('\n', " ");

                let session_marker = if all { format!(" @{}", &sid[..8.min(sid.len())]) } else { String::new() };
                println!("[{}] {:?} | {}{} (score: {:.2})", i + 1, r.doc_type, r.title, session_marker, r.score);
                println!("    {}", content_preview);
                if content_preview.len() >= 200 {
                    println!("    ...");
                }
                println!();
            }
        }

        Commands::Server { ref host, port, ref jwt_secret } => {
            use ouroboros::api::server::{ApiServer, ApiServerConfig};

            let secret = jwt_secret
                .clone()
                .or_else(|| std::env::var("JWT_SECRET").ok())
                .unwrap_or_else(|| {
                    println!("Warning: Using default JWT secret. Set JWT_SECRET env var or --jwt-secret for production.");
                    "default_secret_change_in_production".to_string()
                });

            let config = ApiServerConfig {
                host: host.clone(),
                port,
                jwt_secret: secret,
                data_dir: cli.data_dir.clone(),
            };

            let server = ApiServer::new(config);
            println!("Starting API server on {}:{}", host, port);
            server.start().await?;
        }

    }

    Ok(())
}

fn parse_date_string(date_str: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    use chrono::{NaiveDate, NaiveDateTime, TimeZone};

    // Try parsing with time first (YYYY-MM-DD HH:MM:SS)
    if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Ok(chrono::Utc.from_utc_datetime(&dt));
    }

    // Try parsing date only (YYYY-MM-DD)
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        return Ok(chrono::Utc.from_utc_datetime(&dt));
    }

    anyhow::bail!("Invalid date format. Use YYYY-MM-DD or YYYY-MM-DD HH:MM:SS")
}

fn init_project(data_dir: &PathBuf) -> Result<()> {
    // Only create sessions directory at root level
    // tasks/results/contexts are created inside each session
    std::fs::create_dir_all(data_dir.join("sessions"))?;
    Ok(())
}

fn create_orchestrator(cli: &Cli) -> Result<Orchestrator> {
    create_orchestrator_with_config(cli, false)
}

fn create_orchestrator_with_config(cli: &Cli, auto_validate: bool) -> Result<Orchestrator> {
    let config = OrchestratorConfig {
        auto_validate,
        ..OrchestratorConfig::default()
    };

    Orchestrator::new(cli.workdir.clone(), cli.data_dir.clone())
        .map(|o| o.with_config(config))
}
