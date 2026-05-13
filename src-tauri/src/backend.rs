use std::path::PathBuf;
use tokio::process::Command as TokioCommand;
use tokio::time::Duration;

const CONFIG_PATH: &str = "~/.ntd/config.yaml";
const DEFAULT_PORT: u16 = 8088;

pub enum NtdStatus {
    Installed { running: bool, port: u16 },
    NotInstalled,
}

/// Read port from ~/.ntd/config.yaml
fn read_port_from_config() -> u16 {
    let path = expand_tilde(CONFIG_PATH);
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            if let Some(port) = map.get("port").and_then(|v| v.as_u64()) {
                return port as u16;
            }
        }
    }
    DEFAULT_PORT
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(path.trim_start_matches("~/").trim_start_matches('/'));
        }
    }
    PathBuf::from(path)
}

/// Check if ntd is running on the given port
async fn is_ntd_running(port: u16) -> bool {
    tokio::net::TcpStream::connect(format!("localhost:{}", port))
        .await
        .is_ok()
}

fn find_ntd_binary() -> Option<String> {
    // Check if ntd is in PATH
    if std::process::Command::new("which")
        .arg("ntd")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("ntd".to_string());
    }

    // Check common installation locations
    let candidates = [".local/bin/ntd", "bin/ntd"];

    if let Some(home) = dirs::home_dir() {
        for candidate in &candidates {
            let path = home.join(candidate);
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }

    None
}

/// Check ntd installation and running status
pub async fn check_ntd_status() -> NtdStatus {
    let port = read_port_from_config();

    if find_ntd_binary().is_none() {
        return NtdStatus::NotInstalled;
    }

    if is_ntd_running(port).await {
        NtdStatus::Installed { running: true, port }
    } else {
        NtdStatus::Installed { running: false, port }
    }
}

/// Start ntd daemon
pub async fn start_ntd_daemon(port: u16) -> Result<u16, String> {
    let ntd_bin = find_ntd_binary().ok_or("ntd binary not found")?;

    TokioCommand::new(&ntd_bin)
        .args(["daemon", "start"])
        .spawn()
        .map_err(|e| format!("Failed to spawn ntd daemon: {}", e))?;

    // Wait for ntd to be ready
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if is_ntd_running(port).await {
            return Ok(port);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err("ntd daemon failed to start".to_string())
}