use std::sync::Arc;
use tokio::sync::broadcast;
use log::info;

use todo_executor::{adapters, db, handlers};

fn main() {
    // Initialize logger with default level info
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    // Get database path from config directory
    let db_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("aitodo")
        .join("data.db");

    // Ensure the directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Initialize database
    let db = Arc::new(
        db::Database::new(db_path.to_str().unwrap()).expect("Failed to open database")
    );

    // Initialize executor registry with adapters
    let executor_registry = Arc::new(adapters::ExecutorRegistry::new());
    executor_registry.register(adapters::joinai::JoinaiExecutor::new());
    executor_registry.register(adapters::claude_code::ClaudeCodeExecutor::new());

    // List available executors
    let executors = executor_registry.list_executors();
    info!("Available executors: {:?}", executors);

    // Create broadcast channel for events
    let (tx, _rx) = broadcast::channel(100);

    // Create app
    let app = handlers::create_app(db, executor_registry, tx);

    info!("===========================================");
    info!("  Todo Executor Server");
    info!("  Open http://127.0.0.1:8088 in your browser");
    info!("===========================================");

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        use std::os::fd::AsRawFd;

        let std_listener = std::net::TcpListener::bind("127.0.0.1:8088").unwrap();
        // Enable SO_REUSEADDR before anything else to allow quick restart
        unsafe {
            let fd = std_listener.as_raw_fd();
            let optval: libc::c_int = 1;
            libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, &optval as *const libc::c_int as *const libc::c_void, std::mem::size_of::<libc::c_int>() as libc::socklen_t);
        }
        std_listener.set_nonblocking(true).unwrap();
        let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();

        axum::serve(listener, app).await.unwrap();
    });
}
