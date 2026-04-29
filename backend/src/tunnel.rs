use std::fs;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use clap::Subcommand;

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
    },
    /// Stop the running tunnel
    Stop,
    /// Show tunnel status
    Status,
}

pub fn handle_tunnel_command(action: &TunnelAction) {
    let ntd_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ntd");

    let pid_file = ntd_dir.join("tunnel.pid");
    let url_file = ntd_dir.join("tunnel.url");

    match action {
        TunnelAction::Start { tunnel_type } => {
            let tunnel_type = tunnel_type.clone();
            fs::create_dir_all(&ntd_dir).expect("Failed to create .ntd directory");

            if let Ok(old_pid_str) = fs::read_to_string(&pid_file) {
                if let Ok(old_pid) = old_pid_str.trim().parse::<u32>() {
                    if is_process_running(old_pid as i32) {
                        println!("Stopping old tunnel (PID: {})", old_pid);
                        #[cfg(unix)]
                        {
                            if is_process_group_leader(old_pid as i32) {
                                kill_process_group(old_pid as i32);
                                thread::sleep(Duration::from_millis(500));
                                if is_process_running(old_pid as i32) {
                                    kill_process_group_force(old_pid as i32);
                                }
                            } else {
                                kill_process(old_pid as i32);
                                thread::sleep(Duration::from_millis(500));
                                if is_process_running(old_pid as i32) {
                                    kill_process_force(old_pid as i32);
                                }
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

            cleanup_orphan_processes();

            match tunnel_type.as_str() {
                "hostc" => start_hostc_tunnel(&pid_file, &url_file),
                "trycloudflare" => start_cloudflare_tunnel(&pid_file, &url_file),
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
                            if is_process_group_leader(pid as i32) {
                                kill_process_group(pid as i32);
                                thread::sleep(Duration::from_millis(500));
                                if is_process_running(pid as i32) {
                                    kill_process_group_force(pid as i32);
                                }
                            } else {
                                kill_process(pid as i32);
                                thread::sleep(Duration::from_millis(500));
                                if is_process_running(pid as i32) {
                                    kill_process_force(pid as i32);
                                }
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

fn start_hostc_tunnel(pid_file: &PathBuf, url_file: &PathBuf) {
    let output_file = "/tmp/hostc_output.txt";

    let output = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)
        .expect("Failed to create output file");

    let mut cmd = std::process::Command::new("hostc");
    cmd.arg(get_port().to_string())
        .stdout(output.try_clone().expect("Failed to clone output file"))
        .stderr(output);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    let mut child = cmd.spawn().expect("Failed to start hostc. Is hostc installed?");
    let hostc_pid = child.id();
    fs::write(pid_file, hostc_pid.to_string()).expect("Failed to write PID file");

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

    if let Ok(content) = fs::read_to_string(output_file) {
        println!("{}", content);
    }

    if public_url.is_empty() {
        cleanup_child_process(&mut child, hostc_pid);
        fs::remove_file(pid_file).ok();
        fs::remove_file(url_file).ok();
        eprintln!("Error: failed to capture Public URL within 60s");
        std::process::exit(1);
    }

    fs::write(url_file, &public_url).expect("Failed to write URL file");
    println!("\nTunnel PID: {}", hostc_pid);
    println!("Public URL saved to ~/.ntd/tunnel.url");
    println!("Public URL: {}", public_url);
}

fn start_cloudflare_tunnel(pid_file: &PathBuf, url_file: &PathBuf) {
    let output_file = "/tmp/cloudflared_output.txt";

    let output = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)
        .expect("Failed to create output file");

    let mut cmd = std::process::Command::new("cloudflared");
    cmd.arg("tunnel")
        .arg("--url")
        .arg(format!("http://localhost:{}", get_port()))
        .stdout(output.try_clone().expect("Failed to clone output file"))
        .stderr(output);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    let mut child = cmd.spawn().expect("Failed to start cloudflared. Is cloudflared installed?");
    let cloudflared_pid = child.id();
    fs::write(pid_file, cloudflared_pid.to_string()).expect("Failed to write PID file");

    let public_url = poll_for_url(output_file, |line| {
        let trimmed = line.trim();
        if trimmed.starts_with("https://") && trimmed.contains("trycloudflare.com") {
            trimmed.to_string()
        } else {
            String::new()
        }
    });

    if let Ok(content) = fs::read_to_string(output_file) {
        println!("{}", content);
    }

    if public_url.is_empty() {
        cleanup_child_process(&mut child, cloudflared_pid);
        fs::remove_file(pid_file).ok();
        fs::remove_file(url_file).ok();
        eprintln!("Error: failed to capture Public URL within 60s");
        std::process::exit(1);
    }

    fs::write(url_file, &public_url).expect("Failed to write URL file");
    println!("\nTunnel PID: {}", cloudflared_pid);
    println!("Public URL saved to ~/.ntd/tunnel.url");
    println!("Public URL: {}", public_url);
}

fn poll_for_url(output_file: &str, extract: impl Fn(&str) -> String) -> String {
    for _ in 0..120 {
        if let Ok(file) = std::fs::File::open(output_file) {
            let reader = std::io::BufReader::new(file);
            for line in reader.lines() {
                if let Ok(line_content) = line {
                    let url = extract(&line_content);
                    if !url.is_empty() {
                        return url;
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    String::new()
}

fn cleanup_child_process(child: &mut std::process::Child, pid: u32) {
    #[cfg(unix)]
    {
        if is_process_group_leader(pid as i32) {
            kill_process_group(pid as i32);
        } else {
            let _ = child.kill();
        }
    }
    #[cfg(windows)]
    {
        let _ = child.kill();
    }
}

#[cfg(unix)]
pub fn is_process_running(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(unix)]
fn is_process_group_leader(pid: i32) -> bool {
    unsafe { libc::getpgid(pid) == pid as libc::pid_t }
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

    if let Ok(output) = Command::new("pgrep")
        .args(["-f", &format!("hostc {}", get_port())])
        .output()
    {
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids.lines() {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                if is_orphan_process(pid) {
                    kill_process(pid);
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
                    kill_process(pid);
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
                kill_process(pid);
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
                kill_process(pid);
            }
        }
    }
}
