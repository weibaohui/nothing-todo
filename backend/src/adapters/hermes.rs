use std::env;
use std::sync::{Arc, Mutex};

use super::{CodeExecutor, ExecutorType, ParsedLogEntry, ExecutionUsage};
use crate::models::utc_timestamp;

pub struct HermesExecutor {
    path: String,
    usage: Arc<Mutex<Option<ExecutionUsage>>>,
    has_done: Arc<Mutex<bool>>,
}

impl HermesExecutor {
    pub fn new() -> Self {
        let path = env::var("HERMES_PATH")
            .unwrap_or_else(|_| "hermes".to_string());
        Self {
            path,
            usage: Arc::new(Mutex::new(None)),
            has_done: Arc::new(Mutex::new(false)),
        }
    }
}

impl Clone for HermesExecutor {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            usage: self.usage.clone(),
            has_done: self.has_done.clone(),
        }
    }
}

impl Default for HermesExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeExecutor for HermesExecutor {
    fn executor_type(&self) -> ExecutorType {
        ExecutorType::Hermes
    }

    fn executable_path(&self) -> &str {
        &self.path
    }

    fn command_args(&self, message: &str) -> Vec<String> {
        vec![
            "chat".to_string(),
            "-Q".to_string(),
            "--yolo".to_string(),
            "-q".to_string(),
            message.to_string(),
        ]
    }

    fn parse_output_line(&self, line: &str) -> Option<ParsedLogEntry> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Skip banner lines and special formatting
        if trimmed.starts_with("╭") || trimmed.starts_with("│") || trimmed.starts_with("╰") {
            return None;
        }

        // Parse session_id from output: "session_id: <id>"
        if trimmed.starts_with("session_id:") {
            return Some(ParsedLogEntry {
                timestamp: utc_timestamp(),
                log_type: "info".to_string(),
                content: trimmed.to_string(),
                usage: None,
            });
        }

        // Skip status indicators
        if trimmed.starts_with("┊") {
            return None;
        }

        // Skip empty box characters
        if trimmed.chars().all(|c| c == ' ' || c == '━' || c == '│' || c == '╰' || c == '╭') {
            return None;
        }

        Some(ParsedLogEntry {
            timestamp: utc_timestamp(),
            log_type: "text".to_string(),
            content: trimmed.to_string(),
            usage: None,
        })
    }

    fn parse_stderr_line(&self, line: &str) -> Option<ParsedLogEntry> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Classify stderr content by its nature - Hermes often outputs info to stderr
        let log_type = if trimmed.contains("error") || trimmed.contains("Error") || trimmed.contains("ERROR") || trimmed.contains("failed") || trimmed.contains("Failed") {
            "stderr".to_string()
        } else {
            "info".to_string()
        };

        Some(ParsedLogEntry {
            timestamp: utc_timestamp(),
            log_type,
            content: trimmed.to_string(),
            usage: None,
        })
    }

    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String> {
        super::default_final_result_with_think_stripping(logs)
    }

    fn get_usage(&self, _logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
        self.usage.lock().unwrap().clone()
    }

    fn get_model(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_output_line_text() {
        let executor = HermesExecutor::new();
        let entry = executor.parse_output_line("Hello world").unwrap();
        assert_eq!(entry.log_type, "text");
        assert_eq!(entry.content, "Hello world");
    }

    #[test]
    fn test_parse_output_line_empty() {
        let executor = HermesExecutor::new();
        assert!(executor.parse_output_line("").is_none());
        assert!(executor.parse_output_line("   ").is_none());
    }

    #[test]
    fn test_parse_output_line_session_id() {
        let executor = HermesExecutor::new();
        let entry = executor.parse_output_line("session_id: abc123").unwrap();
        assert_eq!(entry.log_type, "info");
        assert!(entry.content.contains("session_id"));
    }

    #[test]
    fn test_parse_output_line_banner() {
        let executor = HermesExecutor::new();
        assert!(executor.parse_output_line("╭─ Hermes ─────────────────────────────────").is_none());
        assert!(executor.parse_output_line("│ some text").is_none());
    }

    #[test]
    fn test_get_final_result_with_text() {
        let executor = HermesExecutor::new();
        let logs = vec![
            ParsedLogEntry::new("text", "  hello world  "),
        ];
        assert_eq!(executor.get_final_result(&logs), Some("hello world".to_string()));
    }

    #[test]
    fn test_get_usage_before_tokens() {
        let executor = HermesExecutor::new();
        assert!(executor.get_usage(&[]).is_none());
    }

    #[test]
    fn test_command_args() {
        let executor = HermesExecutor::new();
        let args = executor.command_args("do something");
        assert_eq!(args, vec!["chat", "-Q", "--yolo", "-q", "do something"]);
    }

    #[test]
    fn test_executor_type() {
        let executor = HermesExecutor::new();
        assert_eq!(executor.executor_type(), ExecutorType::Hermes);
    }
}
