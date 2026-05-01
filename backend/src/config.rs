//! Unified configuration management.
//!
//! Config file location: `~/.ntd/config.yaml`
//!
//! All components (server, CLI, tunnel, executors) read their settings from this module.
//! No direct environment variable reads — route everything through Config.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Default port.
pub const DEFAULT_PORT: u16 = 8088;
/// Default host.
pub const DEFAULT_HOST: &str = "0.0.0.0";
/// Default executor paths (binary names).
pub const DEFAULT_EXECUTOR_PATH: &str = ""; // use binary name directly

/// Top-level configuration, persisted to `~/.ntd/config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Server port (default: 8088)
    pub port: u16,
    /// Server host (default: 0.0.0.0)
    pub host: String,
    /// Database file path (default: ~/.ntd/data.db)
    pub db_path: String,
    /// Log level (default: INFO)
    pub log_level: String,
    /// Executor binary paths. Empty string means use default binary name.
    pub executors: ExecutorPaths,
}

/// Paths for each supported executor binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutorPaths {
    pub opencode: String,
    pub hermes: String,
    pub joinai: String,
    pub claude_code: String,
    pub codebuddy: String,
    pub kimi: String,
    pub atomcode: String,
    pub codex: String,
}

impl Default for ExecutorPaths {
    fn default() -> Self {
        Self {
            opencode: "opencode".to_string(),
            hermes: "hermes".to_string(),
            joinai: "joinai".to_string(),
            claude_code: "claude".to_string(),
            codebuddy: "codebuddy".to_string(),
            kimi: "kimi".to_string(),
            atomcode: "atomcode".to_string(),
            codex: "codex".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            port: DEFAULT_PORT,
            host: DEFAULT_HOST.to_string(),
            db_path: home.join(".ntd").join("data.db").to_string_lossy().to_string(),
            log_level: "INFO".to_string(),
            executors: ExecutorPaths::default(),
        }
    }
}

impl Config {
    /// Load config from `~/.ntd/config.yaml`.
    /// Creates the file with defaults if it doesn't exist.
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            let cfg = Config::default();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            if let Ok(yaml) = serde_yaml::to_string(&cfg) {
                if let Err(e) = std::fs::write(&path, yaml) {
                    eprintln!("Warning: failed to write config.yaml ({}), using in-memory defaults", e);
                }
            }
            return cfg;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let mut cfg = serde_yaml::from_str::<Config>(&content).unwrap_or_else(|e| {
                    eprintln!("Warning: failed to parse config.yaml ({}), using defaults", e);
                    Config::default()
                });
                cfg.normalize_paths();
                cfg
            }
            Err(e) => {
                eprintln!("Warning: failed to read config.yaml ({}), using defaults", e);
                Config::default()
            }
        }
    }

    /// Normalize paths: convert relative paths to absolute paths based on home directory.
    fn normalize_paths(&mut self) {
        if !PathBuf::from(&self.db_path).is_absolute() {
            if let Some(home) = dirs::home_dir() {
                self.db_path = home.join(&self.db_path).to_string_lossy().to_string();
            }
        }
    }

    /// Get the server URL string, e.g. "http://localhost:8088".
    pub fn server_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Path to the config file.
    fn config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".ntd").join("config.yaml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_port() {
        let cfg = Config::default();
        assert_eq!(cfg.port, 8088);
    }

    #[test]
    fn test_server_url() {
        let cfg = Config { port: 9090, ..Default::default() };
        assert_eq!(cfg.server_url(), "http://localhost:9090");
    }

    #[test]
    fn test_round_trip() {
        let cfg = Config { port: 1234, ..Default::default() };
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        let restored: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(restored.port, 1234);
        assert!(restored.db_path.contains(".ntd/data.db"));
    }
}
