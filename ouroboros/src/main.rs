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

        Commands::Plan { ref goal } => {
            let mut orch = create_orchestrator(&cli)?;
            let task_ids = orch.plan(goal).await?;
            println!("Created {} tasks:", task_ids.len());
            for id in task_ids {
                println!("  - {}", id);
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

        Commands::Run { ref task_id, all } => {
            let mut orch = create_orchestrator(&cli)?;

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

    }

    Ok(())
}

fn init_project(data_dir: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(data_dir.join("tasks"))?;
    std::fs::create_dir_all(data_dir.join("results"))?;
    std::fs::create_dir_all(data_dir.join("contexts"))?;
    Ok(())
}

fn create_orchestrator(cli: &Cli) -> Result<Orchestrator> {
    let config = OrchestratorConfig::default();

    Orchestrator::new(cli.workdir.clone(), cli.data_dir.clone())
        .map(|o| o.with_config(config))
}
