use std::env;
use std::sync::{Arc, Mutex};

use super::{CodeExecutor, ExecutorType, ParsedLogEntry, ExecutionUsage};
use crate::models::utc_timestamp;

pub struct KimiExecutor {
    path: String,
    usage: Arc<Mutex<Option<ExecutionUsage>>>,
}

impl KimiExecutor {
    pub fn new() -> Self {
        let path = env::var("KIMI_PATH")
            .unwrap_or_else(|_| "kimi".to_string());
        Self {
            path,
            usage: Arc::new(Mutex::new(None)),
        }
    }
}

impl Clone for KimiExecutor {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            usage: self.usage.clone(),
        }
    }
}

impl Default for KimiExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeExecutor for KimiExecutor {
    fn executor_type(&self) -> ExecutorType {
        ExecutorType::Kimi
    }

    fn executable_path(&self) -> &str {
        &self.path
    }

    fn command_args(&self, message: &str) -> Vec<String> {
        vec![
            "--print".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "-p".to_string(),
            message.to_string(),
        ]
    }

    fn command_args_with_session(&self, message: &str, session_id: Option<&str>) -> Vec<String> {
        let mut args = self.command_args(message);
        if let Some(sid) = session_id {
            args.push("-S".to_string());
            args.push(sid.to_string());
        }
        args
    }

    fn parse_output_line(&self, line: &str) -> Option<ParsedLogEntry> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Skip non-JSON lines
        if !trimmed.starts_with('{') {
            return None;
        }

        let json = match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let role = json.get("role").and_then(|v| v.as_str())?;

        // Skip assistant messages with tool_calls (tool call requests, not results)
        if role == "assistant" {
            if json.get("tool_calls").is_some() {
                return None;
            }
            // Final assistant message with text content
            if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
                for item in content {
                    if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                            return Some(ParsedLogEntry {
                                timestamp: utc_timestamp(),
                                log_type: "text".to_string(),
                                content: text.to_string(),
                                usage: None,
                            });
                        }
                    }
                }
            }
            return None;
        }

        // Tool result: extract the actual result (skip <system> messages)
        if role == "tool" {
            if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
                for item in content {
                    if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                            // Skip <system> messages, keep actual results
                            if text.starts_with("<system>") {
                                continue;
                            }
                            return Some(ParsedLogEntry {
                                timestamp: utc_timestamp(),
                                log_type: "tool".to_string(),
                                content: text.to_string(),
                                usage: None,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn parse_stderr_line(&self, line: &str) -> Option<ParsedLogEntry> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("To resume this session:") {
            return None;
        }
        Some(ParsedLogEntry {
            timestamp: utc_timestamp(),
            log_type: "stderr".to_string(),
            content: trimmed.to_string(),
            usage: None,
        })
    }

    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String> {
        logs.iter()
            .rev()
            .find(|l| l.log_type == "text")
            .map(|l| l.content.clone())
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
    fn test_command_args() {
        let executor = KimiExecutor::new();
        let args = executor.command_args("do something");
        assert_eq!(args, vec!["--print", "--output-format", "stream-json", "-p", "do something"]);
    }

    #[test]
    fn test_command_args_with_session() {
        let executor = KimiExecutor::new();
        let args = executor.command_args_with_session("continue task", Some("abc123"));
        assert_eq!(args, vec!["--print", "--output-format", "stream-json", "-p", "continue task", "-S", "abc123"]);
    }

    #[test]
    fn test_executor_type() {
        let executor = KimiExecutor::new();
        assert_eq!(executor.executor_type(), ExecutorType::Kimi);
    }

    #[test]
    fn test_parse_output_line_assistant_text() {
        let executor = KimiExecutor::new();
        let json = r#"{"role":"assistant","content":[{"type":"text","text":"Hello world"}]}"#;
        let entry = executor.parse_output_line(json).unwrap();
        assert_eq!(entry.log_type, "text");
        assert_eq!(entry.content, "Hello world");
    }

    #[test]
    fn test_parse_output_line_tool_result() {
        let executor = KimiExecutor::new();
        let json = r#"{"role":"tool","content":[{"type":"text","text":"Tue Apr 28 07:59:16 PDT 2026"}]}"#;
        let entry = executor.parse_output_line(json).unwrap();
        assert_eq!(entry.log_type, "tool");
        assert_eq!(entry.content, "Tue Apr 28 07:59:16 PDT 2026");
    }

    #[test]
    fn test_parse_output_line_skip_tool_call_request() {
        let executor = KimiExecutor::new();
        let json = r#"{"role":"assistant","content":[{"type":"think","think":"..."}],"tool_calls":[{"type":"function","id":"call_1","function":{"name":"Shell","arguments":"{}"}}]}"#;
        assert!(executor.parse_output_line(json).is_none());
    }

    #[test]
    fn test_parse_output_line_skip_resume() {
        let executor = KimiExecutor::new();
        let line = "To resume this session: kimi -r abc123";
        assert!(executor.parse_output_line(line).is_none());
    }

    #[test]
    fn test_parse_output_line_skip_system_message() {
        let executor = KimiExecutor::new();
        let json = r#"{"role":"tool","content":[{"type":"text","text":"<system>Command executed successfully.</system>"},{"type":"text","text":"Tue Apr 28"}]}"#;
        let entry = executor.parse_output_line(json).unwrap();
        assert_eq!(entry.log_type, "tool");
        assert_eq!(entry.content, "Tue Apr 28");
    }
}
