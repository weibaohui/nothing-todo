use std::sync::Arc;
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::info;

use ntd::{adapters, cli, db, handlers, scheduler::TodoScheduler, task_manager::TaskManager, tunnel};

/// ntd - Nothing Todo
#[derive(Parser)]
#[command(name = "ntd", about = "AI Todo CLI", version)]
struct Cli {
    /// API server URL (default: http://localhost:8088)
    #[arg(long, default_value = "http://localhost:8088")]
    server: String,

    /// Output format
    #[arg(short, long, default_value = "json", value_enum)]
    output: OutputFormat,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Json,
    Pretty,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version info
    Version,
    /// Upgrade ntd to the latest version via npm
    Upgrade,
    /// Manage tunnels
    Tunnel {
        #[command(subcommand)]
        action: tunnel::TunnelAction,
    },
    /// Start the API server
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },
    /// Todo management
    Todo {
        #[command(subcommand)]
        action: cli::TodoAction,
    },
    /// Tag management
    Tag {
        #[command(subcommand)]
        action: cli::TagAction,
    },
    /// Global statistics
    Stats,
}

#[derive(Subcommand)]
enum ServerAction {
    /// Start the API server
    Start,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Version) => {
            println!("ntd {}", env!("CARGO_PKG_VERSION"));
            println!("git: {}", option_env!("VERGEN_GIT_SHA").unwrap_or("unknown"));
            if let Some(desc) = option_env!("VERGEN_GIT_DESCRIBE") {
                println!("tag: {}", desc);
            }
            return;
        }
        Some(Commands::Upgrade) => {
            println!("Upgrading ntd...");
            let status = std::process::Command::new("npm")
                .args(["install", "-g", "@weibaohui/nothing-todo@latest"])
                .status()
                .expect("Failed to run npm. Is npm installed?");
            if status.success() {
                println!("Upgrade completed successfully!");
            } else {
                eprintln!("Upgrade failed.");
                std::process::exit(1);
            }
            return;
        }
        Some(Commands::Tunnel { action }) => {
            tunnel::handle_tunnel_command(action);
            return;
        }
        Some(Commands::Server { action: ServerAction::Start }) => {
            println!("Starting ntd server...");
            run_server().await;
            return;
        }
        Some(Commands::Todo { action }) => {
            let cli = cli::Cli {
                server: cli.server.clone(),
                output: output_to_cli(&cli.output),
                command: cli::Commands::Todo { action: action.clone() },
            };
            if let Err(e) = cli::run_command(&cli).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            return;
        }
        Some(Commands::Tag { action }) => {
            let cli = cli::Cli {
                server: cli.server.clone(),
                output: output_to_cli(&cli.output),
                command: cli::Commands::Tag { action: action.clone() },
            };
            if let Err(e) = cli::run_command(&cli).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            return;
        }
        Some(Commands::Stats) => {
            let cli = cli::Cli {
                server: cli.server.clone(),
                output: output_to_cli(&cli.output),
                command: cli::Commands::Stats,
            };
            if let Err(e) = cli::run_command(&cli).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            return;
        }
        None => {
            // No subcommand: start server by default
            println!("Starting ntd server...");
            run_server().await;
        }
    }
}

fn output_to_cli(output: &OutputFormat) -> cli::OutputFormat {
    match output {
        OutputFormat::Json => cli::OutputFormat::Json,
        OutputFormat::Pretty => cli::OutputFormat::Pretty,
    }
}

async fn run_server() {
    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(tracing::Level::INFO);

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(true)
        .with_timer(tracing_subscriber::fmt::time::time())
        .init();

    let db_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ntd")
        .join("data.db");

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db = Arc::new(
        db::Database::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to open database")
    );

    db.cleanup_orphan_execution_records().await;

    let executor_registry = Arc::new(adapters::ExecutorRegistry::new());
    executor_registry.register(adapters::joinai::JoinaiExecutor::new());
    executor_registry.register(adapters::claude_code::ClaudeCodeExecutor::new());
    executor_registry.register(adapters::codebuddy::CodebuddyExecutor::new());
    executor_registry.register(adapters::opencode::OpencodeExecutor::new());
    executor_registry.register(adapters::atomcode::AtomcodeExecutor::new());
    executor_registry.register(adapters::hermes::HermesExecutor::new());
    executor_registry.register(adapters::kimi::KimiExecutor::new());

    let executors = executor_registry.list_executors();
    info!("Available executors: {:?}", executors);

    let (tx, _rx) = broadcast::channel(100);
    let task_manager = Arc::new(TaskManager::new());

    let scheduler = Arc::new({
        let sched = TodoScheduler::new().await.expect("Failed to create scheduler");
        sched.load_from_db(db.clone(), executor_registry.clone(), tx.clone(), task_manager.clone()).await.expect("Failed to load scheduled tasks");
        sched.start().await.expect("Failed to start scheduler");
        sched
    });

    let app = handlers::create_app(db, executor_registry, tx, scheduler, task_manager);

    let port = std::env::var("NTD_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8088);

    info!("===========================================");
    info!("  Nothing Todo (ntd)");
    info!("  Open http://localhost:{} in your browser", port);
    info!("===========================================");

    let std_listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();

    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        let optval: libc::c_int = 1;
        unsafe {
            libc::setsockopt(
                std_listener.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_REUSEADDR,
                &optval as *const libc::c_int as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
        }
    }

    std_listener.set_nonblocking(true).unwrap();
    let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();

    axum::serve(listener, app).await.unwrap();
}
