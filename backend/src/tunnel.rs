use std::fs;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use clap::Subcommand;
use serde_json;

fn get_port() -> u16 {
    static PORT: OnceLock<u16> = OnceLock::new();
    *PORT.get_or_init(|| crate::config::Config::load().port)
}

#[derive(Subcommand)]
pub enum TunnelAction {
    /// Start a tunnel
    Start {
        /// Tunnel type (hostc, trycloudflare)
        #[arg(long = "type", default_value = "hostc")]
        tunnel_type: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Stop the running tunnel
    Stop,
    /// Show tunnel status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn handle_tunnel_command(action: &TunnelAction) {
    let ntd_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ntd");

    let pid_file = ntd_dir.join("tunnel.pid");
    let url_file = ntd_dir.join("tunnel.url");

    match action {
        TunnelAction::Start { tunnel_type, json } => {
            let tunnel_type = tunnel_type.clone();
            if let Err(e) = fs::create_dir_all(&ntd_dir) {
                if *json {
                    let err = serde_json::json!({"error": true, "message": format!("Failed to create .ntd directory: {}", e)});
                    eprintln!("{}", serde_json::to_string(&err).unwrap());
                } else {
                    eprintln!("Failed to create .ntd directory: {}", e);
                }
                std::process::exit(1);
            }

            // 停止已存在的 tunnel
            if let Ok(old_pid_str) = fs::read_to_string(&pid_file) {
                if let Ok(old_pid) = old_pid_str.trim().parse::<u32>() {
                    if is_process_running(old_pid as i32) {
                        if !json {
                            println!("Stopping old tunnel (PID: {})", old_pid);
                        }
                        kill_process_safe(old_pid as i32);
                    }
                }
            }

            cleanup_orphan_processes();

            match tunnel_type.as_str() {
                "hostc" => start_hostc_tunnel(&pid_file, &url_file, *json),
                "trycloudflare" => start_cloudflare_tunnel(&pid_file, &url_file, *json),
                _ => {
                    if *json {
                        let err = serde_json::json!({"error": true, "message": format!("unsupported tunnel type '{}'", tunnel_type)});
                        eprintln!("{}", serde_json::to_string(&err).unwrap());
                    } else {
                        eprintln!("Error: unsupported tunnel type '{}'", tunnel_type);
                        eprintln!("Supported types: hostc, trycloudflare");
                    }
                    std::process::exit(1);
                }
            }
        }
        TunnelAction::Stop => {
            if let Ok(pid_str) = fs::read_to_string(&pid_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if is_process_running(pid as i32) {
                        println!("Stopping tunnel (PID: {})", pid);
                        kill_process_safe(pid as i32);
                        fs::remove_file(&pid_file).ok();
                        fs::remove_file(&url_file).ok();
                        println!("Tunnel stopped");
                    } else {
                        eprintln!("No tunnel is running");
                    }
                } else {
                    eprintln!("No tunnel is running");
                }
            } else {
                eprintln!("No tunnel is running");
            }
        }
        TunnelAction::Status { json } => {
            let mut status = serde_json::Map::new();
            status.insert("running".to_string(), serde_json::Value::Bool(false));

            if let Ok(pid_str) = fs::read_to_string(&pid_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if is_process_running(pid as i32) {
                        status.insert("running".to_string(), serde_json::Value::Bool(true));
                        status.insert("pid".to_string(), serde_json::Value::Number(pid.into()));
                        if let Ok(url) = fs::read_to_string(&url_file) {
                            let url = url.trim().to_string();
                            status.insert("url".to_string(), serde_json::Value::String(url.clone()));
                            if !json {
                                println!("Tunnel is running (PID: {})", pid);
                                println!("Public URL: {}", url);
                            }
                        } else if !json {
                            println!("Tunnel is running (PID: {})", pid);
                        }
                    } else {
                        status.insert("stale_pid".to_string(), serde_json::Value::Bool(true));
                        fs::remove_file(&pid_file).ok();
                        if !json {
                            println!("Tunnel is not running (stale PID file)");
                        }
                    }
                } else {
                    fs::remove_file(&pid_file).ok();
                    fs::remove_file(&url_file).ok();
                    if !json {
                        println!("Tunnel state is invalid (bad PID file)");
                    }
                }
            } else if !json {
                println!("No tunnel is running");
            }

            if *json {
                println!("{}", serde_json::to_string(&status).unwrap());
            }
        }
    }
}

fn start_hostc_tunnel(pid_file: &PathBuf, url_file: &PathBuf, json: bool) {
    let output_file = "/tmp/hostc_output.txt";

    let output = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            std::process::exit(1);
        }
    };

    let mut cmd = std::process::Command::new("hostc");
    match output.try_clone() {
        Ok(cloned) => { cmd.stdout(cloned).stderr(output); }
        Err(e) => {
            eprintln!("Failed to clone output file: {}", e);
            std::process::exit(1);
        }
    }

    // 使用 command-group 创建进程组，确保可以安全清理进程树
    let mut child = match command_group::CommandGroup::group_spawn(&mut cmd) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start hostc. Is hostc installed? Error: {}", e);
            std::process::exit(1);
        }
    };

    let hostc_pid = child.id();
    if let Err(e) = fs::write(pid_file, hostc_pid.to_string()) {
        eprintln!("Failed to write PID file: {}", e);
        std::process::exit(1);
    }

    let public_url = poll_for_url(output_file, |line| {
        if line.contains("Public URL:") {
            line.split("Public URL:")
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    if !json {
        if let Ok(content) = fs::read_to_string(output_file) {
            println!("{}", content);
        }
    }

    if public_url.is_empty() {
        // 使用 command-group 安全杀死进程组
        let _ = child.kill();
        fs::remove_file(pid_file).ok();
        fs::remove_file(url_file).ok();
        if json {
            let err = serde_json::json!({"error": true, "message": "failed to capture Public URL within 60s"});
            eprintln!("{}", serde_json::to_string(&err).unwrap());
        } else {
            eprintln!("Error: failed to capture Public URL within 60s");
        }
        std::process::exit(1);
    }

    if let Err(e) = fs::write(url_file, &public_url) {
        if json {
            let err = serde_json::json!({"error": true, "message": format!("Failed to write URL file: {}", e)});
            eprintln!("{}", serde_json::to_string(&err).unwrap());
        } else {
            eprintln!("Failed to write URL file: {}", e);
        }
        std::process::exit(1);
    }
    if json {
        let result = serde_json::json!({
            "pid": hostc_pid,
            "url": public_url,
            "type": "hostc",
        });
        println!("{}", serde_json::to_string(&result).unwrap());
    } else {
        println!("\nTunnel PID: {}", hostc_pid);
        println!("Public URL saved to ~/.ntd/tunnel.url");
        println!("Public URL: {}", public_url);
    }
}

fn start_cloudflare_tunnel(pid_file: &PathBuf, url_file: &PathBuf, json: bool) {
    let output_file = "/tmp/cloudflared_output.txt";

    let output = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            std::process::exit(1);
        }
    };

    let mut cmd = std::process::Command::new("cloudflared");
    match output.try_clone() {
        Ok(cloned) => { cmd.stdout(cloned).stderr(output); }
        Err(e) => {
            eprintln!("Failed to clone output file: {}", e);
            std::process::exit(1);
        }
    }

    // 使用 command-group 创建进程组，确保可以安全清理进程树
    let mut child = match command_group::CommandGroup::group_spawn(&mut cmd) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start cloudflared. Is cloudflared installed? Error: {}", e);
            std::process::exit(1);
        }
    };

    let cloudflared_pid = child.id();
    if let Err(e) = fs::write(pid_file, cloudflared_pid.to_string()) {
        eprintln!("Failed to write PID file: {}", e);
        std::process::exit(1);
    }

    let public_url = poll_for_url(output_file, |line| {
        let trimmed = line.trim();
        if trimmed.starts_with("https://") && trimmed.contains("trycloudflare.com") {
            trimmed.to_string()
        } else {
            String::new()
        }
    });

    if !json {
        if let Ok(content) = fs::read_to_string(output_file) {
            println!("{}", content);
        }
    }

    if public_url.is_empty() {
        // 使用 command-group 安全杀死进程组
        let _ = child.kill();
        fs::remove_file(pid_file).ok();
        fs::remove_file(url_file).ok();
        if json {
            let err = serde_json::json!({"error": true, "message": "failed to capture Public URL within 60s"});
            eprintln!("{}", serde_json::to_string(&err).unwrap());
        } else {
            eprintln!("Error: failed to capture Public URL within 60s");
        }
        std::process::exit(1);
    }

    if let Err(e) = fs::write(url_file, &public_url) {
        if json {
            let err = serde_json::json!({"error": true, "message": format!("Failed to write URL file: {}", e)});
            eprintln!("{}", serde_json::to_string(&err).unwrap());
        } else {
            eprintln!("Failed to write URL file: {}", e);
        }
        std::process::exit(1);
    }
    if json {
        let result = serde_json::json!({
            "pid": cloudflared_pid,
            "url": public_url,
            "type": "trycloudflare",
        });
        println!("{}", serde_json::to_string(&result).unwrap());
    } else {
        println!("\nTunnel PID: {}", cloudflared_pid);
        println!("Public URL saved to ~/.ntd/tunnel.url");
        println!("Public URL: {}", public_url);
    }
}

fn poll_for_url(output_file: &str, extract: impl Fn(&str) -> String) -> String {
    for _ in 0..120 {
        if let Ok(file) = std::fs::File::open(output_file) {
            let reader = std::io::BufReader::new(file);
            for line_content in reader.lines().flatten() {
                let url = extract(&line_content);
                if !url.is_empty() {
                    return url;
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    String::new()
}

/// 安全地杀死进程及其进程组
/// 使用 SIGTERM 优雅退出，等待后如果仍在运行则使用 SIGKILL
fn kill_process_safe(pid: i32) {
    #[cfg(unix)]
    {
        // 先发送 SIGTERM
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }
        thread::sleep(Duration::from_millis(500));

        // 如果仍在运行，发送 SIGKILL
        if is_process_running(pid) {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
            thread::sleep(Duration::from_millis(200));
        }
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
}

#[cfg(unix)]
pub fn is_process_running(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(windows)]
pub fn is_process_running(pid: i32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

#[cfg(unix)]
fn cleanup_orphan_processes() {
    use std::process::Command;

    if let Ok(output) = Command::new("pgrep")
        .args(["-f", &format!("hostc {}", get_port())])
        .output()
    {
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids.lines() {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                if is_orphan_process(pid) {
                    kill_process_safe(pid);
                }
            }
        }
    }

    if let Ok(output) = Command::new("pgrep")
        .args(["-f", &format!("cloudflared tunnel.*{}", get_port())])
        .output()
    {
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids.lines() {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                if is_orphan_process(pid) {
                    kill_process_safe(pid);
                }
            }
        }
    }
}

#[cfg(unix)]
fn is_orphan_process(pid: i32) -> bool {
    use std::process::Command;

    if let Ok(output) = Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
    {
        let ppid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(ppid) = ppid_str.parse::<i32>() {
            return ppid == 1;
        }
    }
    false
}

#[cfg(windows)]
fn cleanup_orphan_processes() {
    use std::process::Command;

    if let Ok(output) = Command::new("wmic")
        .args(["process", "where", &format!("commandline like '%hostc {}%'", get_port()), "get", "processid"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().skip(1) {
            if let Ok(pid) = line.trim().parse::<i32>() {
                kill_process_safe(pid);
            }
        }
    }

    if let Ok(output) = Command::new("wmic")
        .args(["process", "where", &format!("commandline like '%cloudflared%{}%'", get_port()), "get", "processid"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().skip(1) {
            if let Ok(pid) = line.trim().parse::<i32>() {
                kill_process_safe(pid);
            }
        }
    }
}
