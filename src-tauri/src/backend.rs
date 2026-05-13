use std::path::PathBuf;
use tokio::process::Command;
use tokio::time::Duration;

pub async fn spawn_backend() -> Result<u16, String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?;

    let backend_bin = find_backend_binary(&exe_path)?;
    let port = find_available_port().await?;

    let mut child = Command::new(&backend_bin)
        .args(["server", "start", "--port", &port.to_string()])
        .spawn()
        .map_err(|e| format!("Failed to spawn backend: {}", e))?;

    // Wait for backend to be ready
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(10) {
        if tokio::net::TcpStream::connect(format!("localhost:{}", port))
            .await
            .is_ok()
        {
            return Ok(port);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let _ = child.kill().await;
    Err("Backend failed to start".to_string())
}

fn find_backend_binary(exe_path: &PathBuf) -> Result<PathBuf, String> {
    // Development: use locally-built binary
    let dev_path = PathBuf::from("../../backend/target/release/ntd");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    // Release: binary is bundled alongside Tauri app
    if let Some(parent) = exe_path.parent() {
        let backend_path = parent.join("ntd");
        if backend_path.exists() {
            return Ok(backend_path);
        }
    }

    Err("Could not find ntd backend binary".to_string())
}

async fn find_available_port() -> Result<u16, String> {
    for port in 8089..8189 {
        if tokio::net::TcpStream::connect(format!("localhost:{}", port))
            .await
            .is_err()
        {
            return Ok(port);
        }
    }
    Err("No available port found".to_string())
}