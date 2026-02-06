use std::path::PathBuf;
use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "ouroboros")]
#[command(about = "Cross-session search and knowledge management", long_about = None)]
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

    /// Manage work sessions
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
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

        /// Filter by start date (format: YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// Filter by end date (format: YYYY-MM-DD)
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

        /// JWT secret key
        #[arg(long)]
        jwt_secret: Option<String>,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List all sessions
    List,

    /// Show current session info
    Current,

    /// Switch to a different session
    Switch {
        /// Session ID or prefix
        session_id: String,
    },

    /// Create a new session
    New {
        /// Session goal/description
        goal: String,

        /// Optional label
        #[arg(short, long)]
        label: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "ouroboros=info".into())
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            std::fs::create_dir_all(cli.data_dir.join("sessions"))?;
            println!("Initialized at {:?}", cli.data_dir);
        }

        Commands::Sessions { ref action } => {
            use ouroboros::work_session::{WorkSessionManager, WorkSessionStatus};

            let mgr = WorkSessionManager::new(&cli.data_dir)?;

            match action {
                SessionAction::List => {
                    let sessions = mgr.list_sessions()?;
                    if sessions.is_empty() {
                        println!("No sessions found.");
                    } else {
                        println!("Sessions:");
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
                            let date = s.created_at.format("%Y-%m-%d %H:%M").to_string();
                            let goal_short: String = s.goal.chars().take(40).collect();
                            let goal_display = if s.goal.chars().count() > 40 {
                                format!("{}...", goal_short)
                            } else {
                                s.goal.clone()
                            };
                            let short_id = &s.id[..10.min(s.id.len())];
                            println!("{} {}{}  {}  {}", status_icon, short_id, label_str, date, goal_display);
                        }
                    }
                }

                SessionAction::Current => {
                    if let Ok(Some(session)) = mgr.current_session() {
                        println!("Current: {}", session.id);
                        println!("  Goal:    {}", session.goal);
                        println!("  Status:  {:?}", session.status);
                        println!("  Created: {}", session.created_at.format("%Y-%m-%d %H:%M"));
                        if let Some(label) = &session.label {
                            println!("  Label:   {}", label);
                        }
                    } else {
                        println!("No current session.");
                    }
                }

                SessionAction::Switch { ref session_id } => {
                    mgr.switch_session(session_id)?;
                    println!("Switched to: {}", session_id);
                }

                SessionAction::New { ref goal, ref label } => {
                    let session = mgr.create_session(goal, label.clone())?;
                    println!("Created session: {}", session.id);
                    println!("  Goal: {}", session.goal);
                }
            }
        }

        Commands::Search { ref query, ref doc_type, ref session, all, limit, ref from, ref to } => {
            use ouroboros::search::{SearchEngine, SearchOptions, DocumentType, SearchResult};
            use ouroboros::work_session::WorkSessionManager;

            let session_mgr = WorkSessionManager::new(&cli.data_dir)?;

            // Collect search index paths
            let search_paths: Vec<(PathBuf, Option<String>)> = if all {
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
                if let Ok(Some(current)) = session_mgr.current_session() {
                    let path = cli.data_dir.join("sessions").join(&current.id).join("search_index");
                    if path.exists() {
                        vec![(path, Some(current.id.clone()))]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            };

            if search_paths.is_empty() {
                println!("No search index found.");
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
                        println!("Unknown document type: {}", dtype);
                        return Ok(());
                    }
                };
                options = options.with_doc_type(dt);
            }

            if let Some(ref from_str) = from {
                if let Ok(date) = parse_date_string(from_str) {
                    options = options.with_date_from(date);
                }
            }

            if let Some(ref to_str) = to {
                if let Ok(date) = parse_date_string(to_str) {
                    options = options.with_date_to(date);
                }
            }

            // Search and collect results
            let mut all_results: Vec<(SearchResult, String)> = Vec::new();

            for (search_path, session_id) in &search_paths {
                let engine = SearchEngine::keyword_reader_only(search_path)?;
                let results = engine.search(query, &options).await?;
                let sid = session_id.clone().unwrap_or_else(|| "default".to_string());
                for r in results {
                    all_results.push((r, sid.clone()));
                }
            }

            all_results.sort_by(|a, b| b.0.score.partial_cmp(&a.0.score).unwrap_or(std::cmp::Ordering::Equal));
            all_results.truncate(limit);

            if all_results.is_empty() {
                println!("No results for: \"{}\"", query);
                return Ok(());
            }

            println!("Results for \"{}\" ({} found):", query, all_results.len());
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
                println!();
            }
        }

        Commands::Server { ref host, port, ref jwt_secret } => {
            use ouroboros::api::server::{ApiServer, ApiServerConfig};

            let secret = jwt_secret
                .clone()
                .or_else(|| std::env::var("JWT_SECRET").ok())
                .unwrap_or_else(|| "default_secret".to_string());

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

    if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Ok(chrono::Utc.from_utc_datetime(&dt));
    }

    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        return Ok(chrono::Utc.from_utc_datetime(&dt));
    }

    anyhow::bail!("Invalid date format")
}
