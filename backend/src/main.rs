use std::sync::Arc;
use clap::{Parser, Subcommand};
use tokio::sync::broadcast;
use tracing::info;

use ntd::{adapters, db, handlers, scheduler::TodoScheduler, task_manager::TaskManager};

/// ntd - Nothing Todo
#[derive(Parser)]
#[command(name = "ntd", about = "AI Todo App", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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
        action: TunnelAction,
    },
}

#[derive(Subcommand)]
enum TunnelAction {
    /// Start a tunnel
    Start {
        /// Tunnel type (hostc, trycloudflare)
        #[arg(long = "type", default_value = "hostc")]
        tunnel_type: String,
    },
    /// Stop the running tunnel
    Stop,
    /// Show tunnel status
    Status,
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
            handle_tunnel_command(action);
            return;
        }
        _ => {}
    }

    run_server().await;
}

fn handle_tunnel_command(action: &TunnelAction) {
    use std::fs;
    use std::io::BufRead;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    let ntd_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ntd");

    let pid_file = ntd_dir.join("tunnel.pid");
    let url_file = ntd_dir.join("tunnel.url");

    match action {
        TunnelAction::Start { tunnel_type } => {
            let tunnel_type = tunnel_type.clone();
            // 确保目录存在
            fs::create_dir_all(&ntd_dir).expect("Failed to create .ntd directory");

            // 如果存在旧的 tunnel pid，先杀掉
            if let Ok(old_pid_str) = fs::read_to_string(&pid_file) {
                if let Ok(old_pid) = old_pid_str.trim().parse::<u32>() {
                    if is_process_running(old_pid as i32) {
                        println!("Stopping old tunnel (PID: {})", old_pid);
                        #[cfg(unix)]
                        {
                            kill_process_group(old_pid as i32);
                            thread::sleep(Duration::from_millis(500));
                            if is_process_running(old_pid as i32) {
                                kill_process_group_force(old_pid as i32);
                            }
                        }
                        #[cfg(windows)]
                        {
                            kill_process(old_pid as i32);
                            for _ in 0..3 {
                                if !is_process_running(old_pid as i32) {
                                    break;
                                }
                                thread::sleep(Duration::from_secs(1));
                            }
                            kill_process_force(old_pid as i32);
                        }
                    }
                }
            }

            // 顺手清理任何残留的 hostc 8088 进程（防止脚本异常退出留下的孤儿）
            cleanup_orphan_processes();

            match tunnel_type.as_str() {
                "hostc" => {
                    // 启动 hostc 隧道
                    let output_file = "/tmp/hostc_output.txt";

                    let mut child = std::process::Command::new("hostc");
                    child.arg("8088")
                        .stdout(std::fs::File::create(output_file).expect("Failed to create output file"))
                        .stderr(std::fs::File::create(output_file).expect("Failed to create output file"));

                    #[cfg(unix)]
                    use std::os::unix::process::CommandExt;
                    #[cfg(unix)]
                    let child = unsafe { child.pre_exec(|| {
                        libc::setsid();
                        Ok(())
                    }) };

                    let mut child = child.spawn().expect("Failed to start hostc. Is hostc installed?");
                    let hostc_pid = child.id();
                    fs::write(&pid_file, hostc_pid.to_string()).expect("Failed to write PID file");

                    // 轮询等待 Public URL（最多 60s）
                    let mut public_url = String::new();
                    for _ in 0..120 {
                        if let Ok(file) = std::fs::File::open(output_file) {
                            let reader = std::io::BufReader::new(file);
                            for line in reader.lines() {
                                if let Ok(line_content) = line {
                                    if line_content.contains("Public URL:") {
                                        public_url = line_content
                                            .split("Public URL:")
                                            .nth(1)
                                            .map(|s| s.trim().to_string())
                                            .unwrap_or_default();
                                        if !public_url.is_empty() {
                                            break;
                                        }
                                    }
                                }
                            }
                            if !public_url.is_empty() {
                                break;
                            }
                        }
                        thread::sleep(Duration::from_millis(500));
                    }

                    // 显示 hostc 输出
                    if let Ok(content) = fs::read_to_string(output_file) {
                        println!("{}", content);
                    }

                    if public_url.is_empty() {
                        // 清理已启动的进程
                        let _ = child.kill();
                        fs::remove_file(&pid_file).ok();
                        fs::remove_file(&url_file).ok();
                        eprintln!("Error: failed to capture Public URL within 60s");
                        std::process::exit(1);
                    }

                    fs::write(&url_file, &public_url).expect("Failed to write URL file");

                    println!("\nTunnel PID: {}", hostc_pid);
                    println!("Public URL saved to ~/.ntd/tunnel.url");
                    println!("Public URL: {}", public_url);
                }
                "trycloudflare" => {
                    // 启动 cloudflare 隧道
                    let output_file = "/tmp/cloudflared_output.txt";

                    let mut child = std::process::Command::new("cloudflared");
                    child.arg("tunnel")
                        .arg("--url")
                        .arg("http://localhost:8088")
                        .stdout(std::fs::File::create(output_file).expect("Failed to create output file"))
                        .stderr(std::fs::File::create(output_file).expect("Failed to create output file"));

                    #[cfg(unix)]
                    use std::os::unix::process::CommandExt;
                    #[cfg(unix)]
                    let child = unsafe { child.pre_exec(|| {
                        libc::setsid();
                        Ok(())
                    }) };

                    let mut child = child.spawn().expect("Failed to start cloudflared. Is cloudflared installed?");
                    let cloudflared_pid = child.id();
                    fs::write(&pid_file, cloudflared_pid.to_string()).expect("Failed to write PID file");

                    // 轮询等待 Public URL（最多 60s）
                    let mut public_url = String::new();
                    for _ in 0..120 {
                        if let Ok(file) = std::fs::File::open(output_file) {
                            let reader = std::io::BufReader::new(file);
                            for line in reader.lines() {
                                if let Ok(line_content) = line {
                                    // cloudflared 输出格式: https://xxx.trycloudflare.com
                                    // 查找以 https:// 开头且包含 trycloudflare.com 的行
                                    if line_content.trim().starts_with("https://") && line_content.contains("trycloudflare.com") {
                                        public_url = line_content.trim().to_string();
                                        if !public_url.is_empty() {
                                            break;
                                        }
                                    }
                                }
                            }
                            if !public_url.is_empty() {
                                break;
                            }
                        }
                        thread::sleep(Duration::from_millis(500));
                    }

                    // 显示 cloudflared 输出
                    if let Ok(content) = fs::read_to_string(output_file) {
                        println!("{}", content);
                    }

                    if public_url.is_empty() {
                        // 清理已启动的进程
                        let _ = child.kill();
                        fs::remove_file(&pid_file).ok();
                        fs::remove_file(&url_file).ok();
                        eprintln!("Error: failed to capture Public URL within 60s");
                        std::process::exit(1);
                    }

                    fs::write(&url_file, &public_url).expect("Failed to write URL file");

                    println!("\nTunnel PID: {}", cloudflared_pid);
                    println!("Public URL saved to ~/.ntd/tunnel.url");
                    println!("Public URL: {}", public_url);
                }
                _ => {
                    eprintln!("Error: unsupported tunnel type '{}'", tunnel_type);
                    eprintln!("Supported types: hostc, trycloudflare");
                    std::process::exit(1);
                }
            }
        }
        TunnelAction::Stop => {
            if let Ok(pid_str) = fs::read_to_string(&pid_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if is_process_running(pid as i32) {
                        println!("Stopping tunnel (PID: {})", pid);
                        #[cfg(unix)]
                        {
                            kill_process_group(pid as i32);
                            thread::sleep(Duration::from_millis(500));
                            if is_process_running(pid as i32) {
                                kill_process_group_force(pid as i32);
                            }
                        }
                        #[cfg(windows)]
                        {
                            kill_process(pid as i32);
                            thread::sleep(Duration::from_secs(1));
                            if is_process_running(pid as i32) {
                                kill_process_force(pid as i32);
                            }
                        }
                    }
                    fs::remove_file(&pid_file).ok();
                    fs::remove_file(&url_file).ok();
                    println!("Tunnel stopped");
                } else {
                    eprintln!("No tunnel is running");
                }
            } else {
                eprintln!("No tunnel is running");
            }
        }
        TunnelAction::Status => {
            if let Ok(pid_str) = fs::read_to_string(&pid_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if is_process_running(pid as i32) {
                        println!("Tunnel is running (PID: {})", pid);
                        if let Ok(url) = fs::read_to_string(&url_file) {
                            println!("Public URL: {}", url.trim());
                        }
                    } else {
                        println!("Tunnel is not running (stale PID file)");
                        fs::remove_file(&pid_file).ok();
                    }
                } else {
                    println!("Tunnel state is invalid (bad PID file)");
                    fs::remove_file(&pid_file).ok();
                    fs::remove_file(&url_file).ok();
                }
            } else {
                println!("No tunnel is running");
            }
        }
    }
}

#[cfg(unix)]
fn is_process_running(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(windows)]
fn is_process_running(pid: i32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

#[cfg(unix)]
fn kill_process(pid: i32) {
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
}

#[cfg(unix)]
fn kill_process_group(pid: i32) {
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }
}

#[cfg(unix)]
fn kill_process_group_force(pid: i32) {
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
}

#[cfg(windows)]
fn kill_process(pid: i32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output();
}

#[cfg(unix)]
fn kill_process_force(pid: i32) {
    unsafe {
        libc::kill(pid, libc::SIGKILL);
    }
}

#[cfg(windows)]
fn kill_process_force(pid: i32) {
    kill_process(pid);
}

#[cfg(unix)]
fn cleanup_orphan_processes() {
    use std::process::Command;

    // 清理残留的 hostc 进程
    if let Ok(output) = Command::new("pgrep")
        .args(["-f", "hostc 8088"])
        .output()
    {
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids.lines() {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                // 只清理父进程是 init (pid 1) 或当前进程的孤儿进程
                if is_orphan_process(pid) {
                    kill_process(pid);
                }
            }
        }
    }

    // 清理残留的 cloudflared 进程
    if let Ok(output) = Command::new("pgrep")
        .args(["-f", "cloudflared tunnel.*8088"])
        .output()
    {
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids.lines() {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                // 只清理父进程是 init (pid 1) 或当前进程的孤儿进程
                if is_orphan_process(pid) {
                    kill_process(pid);
                }
            }
        }
    }
}

#[cfg(unix)]
fn is_orphan_process(pid: i32) -> bool {
    use std::process::Command;

    // 获取进程的父进程 ID
    if let Ok(output) = Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
    {
        let ppid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(ppid) = ppid_str.parse::<i32>() {
            // 孤儿进程：父进程是 init (pid 1)
            return ppid == 1;
        }
    }
    false
}

#[cfg(windows)]
fn cleanup_orphan_processes() {
    use std::process::Command;

    if let Ok(output) = Command::new("wmic")
        .args(["process", "where", "commandline like '%hostc 8088%'", "get", "processid"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().skip(1) {
            if let Ok(pid) = line.trim().parse::<i32>() {
                kill_process(pid);
            }
        }
    }

    if let Ok(output) = Command::new("wmic")
        .args(["process", "where", "commandline like '%cloudflared%8088%'", "get", "processid"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().skip(1) {
            if let Ok(pid) = line.trim().parse::<i32>() {
                kill_process(pid);
            }
        }
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

    // Get database path from home directory
    let db_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ntd")
        .join("data.db");

    // Ensure the directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Initialize database
    let db = Arc::new(
        db::Database::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to open database")
    );

    // 清理孤儿执行记录（程序崩溃后状态为running但没有task_id的记录）
    db.cleanup_orphan_execution_records().await;

    // Initialize executor registry with adapters
    let executor_registry = Arc::new(adapters::ExecutorRegistry::new());
    executor_registry.register(adapters::joinai::JoinaiExecutor::new());
    executor_registry.register(adapters::claude_code::ClaudeCodeExecutor::new());
    executor_registry.register(adapters::codebuddy::CodebuddyExecutor::new());
    executor_registry.register(adapters::opencode::OpencodeExecutor::new());
    executor_registry.register(adapters::atomcode::AtomcodeExecutor::new());
    executor_registry.register(adapters::hermes::HermesExecutor::new());
    executor_registry.register(adapters::kimi::KimiExecutor::new());

    // List available executors
    let executors = executor_registry.list_executors();
    info!("Available executors: {:?}", executors);

    // Create broadcast channel for events
    let (tx, _rx) = broadcast::channel(100);

    // Initialize task manager
    let task_manager = Arc::new(TaskManager::new());

    // Initialize scheduler
    let scheduler = Arc::new({
        let sched = TodoScheduler::new().await.expect("Failed to create scheduler");
        sched.load_from_db(db.clone(), executor_registry.clone(), tx.clone(), task_manager.clone()).await.expect("Failed to load scheduled tasks");
        sched.start().await.expect("Failed to start scheduler");
        sched
    });

    // Create app
    let app = handlers::create_app(db, executor_registry, tx, scheduler, task_manager);

    info!("===========================================");
    info!("  Nothing Todo (ntd)");
    info!("  Open http://localhost:8088 in your browser");
    info!("===========================================");

    let std_listener = std::net::TcpListener::bind("0.0.0.0:8088").unwrap();

    // Enable SO_REUSEADDR on Unix to allow quick restart (Windows doesn't need it)
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
