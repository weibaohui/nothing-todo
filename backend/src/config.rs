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
    /// 是否开启自动数据库备份
    pub auto_backup_enabled: bool,
    /// 自动备份 cron 表达式（6 字段，含秒）
    pub auto_backup_cron: String,
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
        Self {
            port: DEFAULT_PORT,
            host: DEFAULT_HOST.to_string(),
            db_path: "~/.ntd/data.db".to_string(),
            log_level: "INFO".to_string(),
            executors: ExecutorPaths::default(),
            auto_backup_enabled: false,
            auto_backup_cron: "0 0 3 * * *".to_string(),
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

    /// Normalize paths: convert ~ and relative paths to absolute paths.
    pub fn normalize_paths(&mut self) {
        self.db_path = Self::normalize_single_path(&self.db_path);
        self.executors.opencode = Self::normalize_single_path(&self.executors.opencode);
        self.executors.hermes = Self::normalize_single_path(&self.executors.hermes);
        self.executors.joinai = Self::normalize_single_path(&self.executors.joinai);
        self.executors.claude_code = Self::normalize_single_path(&self.executors.claude_code);
        self.executors.codebuddy = Self::normalize_single_path(&self.executors.codebuddy);
        self.executors.kimi = Self::normalize_single_path(&self.executors.kimi);
        self.executors.atomcode = Self::normalize_single_path(&self.executors.atomcode);
        self.executors.codex = Self::normalize_single_path(&self.executors.codex);
    }

    fn normalize_single_path(path: &str) -> String {
        if path.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                let relative = path.trim_start_matches('~').trim_start_matches(std::path::MAIN_SEPARATOR);
                return home.join(relative).to_string_lossy().to_string();
            }
        } else if !path.is_empty()
            && !PathBuf::from(path).is_absolute()
            && path.contains(std::path::MAIN_SEPARATOR)
        {
            if let Some(home) = dirs::home_dir() {
                let stripped = path.trim_start_matches("./");
                return home.join(stripped).to_string_lossy().to_string();
            }
        }
        path.to_string()
    }

    /// Get the server URL string, e.g. "http://localhost:8088".
    pub fn server_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Save config to `~/.ntd/config.yaml`.
    /// Uses atomic write (temp file + rename) to avoid corruption on crash.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        let yaml = serde_yaml::to_string(self).map_err(|e| format!("Failed to serialize config: {}", e))?;

        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, yaml).map_err(|e| format!("Failed to write temp config: {}", e))?;
        std::fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename config: {}", e))?;
        Ok(())
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

    #[test]
    fn test_normalize_single_path_tilde_expansion() {
        let home = dirs::home_dir().expect("need home dir for test");
        let result = Config::normalize_single_path("~/bin/joinai");
        let expected = home.join("bin").join("joinai").to_string_lossy().to_string();
        assert_eq!(result, expected, "~ should expand to home directory");
    }

    #[test]
    fn test_normalize_single_path_relative() {
        let home = dirs::home_dir().expect("need home dir for test");
        let result = Config::normalize_single_path("./local/claude");
        assert!(
            result.starts_with(&format!("{}", home.display())),
            "relative path should be resolved to absolute under home"
        );
        assert_ne!(result, "./local/claude", "relative path should be changed");
    }

    #[test]
    fn test_normalize_single_path_bare_command() {
        let result = Config::normalize_single_path("opencode");
        assert_eq!(result, "opencode", "bare command name should be left untouched for PATH lookup");

        let result = Config::normalize_single_path("joinai");
        assert_eq!(result, "joinai", "bare command name should be left untouched for PATH lookup");
    }

    #[test]
    fn test_normalize_single_path_empty() {
        let result = Config::normalize_single_path("");
        assert_eq!(result, "", "empty path should remain empty");
    }

    #[test]
    fn test_normalize_single_path_already_absolute() {
        let result = Config::normalize_single_path("/usr/bin/claude");
        assert_eq!(result, "/usr/bin/claude", "absolute path should not be modified");
    }
}
