use std::path::PathBuf;
use std::process::Command;
use tokio::process::Command as TokioCommand;
use tokio::time::Duration;

const CONFIG_PATH: &str = "~/.ntd/config.yaml";
const DEFAULT_PORT: u16 = 8088;

pub enum NtdStatus {
    Installed { running: bool, port: u16 },
    NotInstalled,
}

fn expand_tilde(path: &str) -> Result<PathBuf, String> {
    if path.starts_with("~/") {
        let home = dirs::home_dir().ok_or("Cannot find home directory")?;
        Ok(home.join(path.trim_start_matches("~/")))
    } else {
        Ok(PathBuf::from(path))
    }
}

fn read_port_from_config() -> u16 {
    let path = match expand_tilde(CONFIG_PATH) {
        Ok(p) => p,
        Err(_) => return DEFAULT_PORT,
    };
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            if let Some(port) = map.get("port").and_then(|v| v.as_u64()) {
                return port as u16;
            }
        }
    }
    DEFAULT_PORT
}

fn is_file_executable(path: &PathBuf) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mode = metadata.permissions().mode();
            return (mode & 0o111) != 0;
        }
        return false;
    }
    #[cfg(not(unix))]
    true
}

fn find_ntd_binary() -> Option<String> {
    // Try ntd --version to verify it's installed and executable
    if Command::new("ntd")
        .arg("--version")
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
            if path.exists() && is_file_executable(&path) {
                if Command::new(&path)
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }

    None
}

async fn is_ntd_responding(port: u16) -> bool {
    let url = format!("http://localhost:{}/health", port);
    match reqwest::get(&url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

async fn is_port_open(port: u16) -> bool {
    tokio::net::TcpStream::connect(format!("localhost:{}", port))
        .await
        .is_ok()
}

pub async fn check_ntd_status() -> NtdStatus {
    let port = read_port_from_config();

    if find_ntd_binary().is_none() {
        return NtdStatus::NotInstalled;
    }

    if is_port_open(port).await && is_ntd_responding(port).await {
        NtdStatus::Installed { running: true, port }
    } else {
        NtdStatus::Installed { running: false, port }
    }
}

pub async fn start_ntd_daemon(port: u16) -> Result<u16, String> {
    let ntd_bin = find_ntd_binary().ok_or("ntd binary not found")?;

    TokioCommand::new(&ntd_bin)
        .args(["daemon", "start"])
        .spawn()
        .map_err(|e| format!("Failed to spawn ntd daemon: {}", e))?;

    // Wait for ntd to be ready with health check
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(30) {
        if is_port_open(port).await && is_ntd_responding(port).await {
            return Ok(port);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err("ntd daemon failed to start or health check timeout".to_string())
}