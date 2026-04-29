use std::sync::Arc;
use parking_lot::Mutex;

use super::{CodeExecutor, ExecutorType, ParsedLogEntry, ExecutionUsage};
use crate::models::utc_timestamp;

pub struct KimiExecutor {
    path: String,
    usage: Arc<Mutex<Option<ExecutionUsage>>>,
}

impl KimiExecutor {
    pub fn new(path: String) -> Self {
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

        // Assistant message: could have tool_calls or text content
        if role == "assistant" {
            // Has tool_calls - this is a tool call request
            if let Some(tool_calls) = json.get("tool_calls").and_then(|v| v.as_array()) {
                for call in tool_calls {
                    if let Some(func) = call.get("function") {
                        let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let args = func.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
                        return Some(ParsedLogEntry {
                            timestamp: utc_timestamp(),
                            log_type: "tool_call".to_string(),
                            content: format!("Calling tool: {} with args: {}", name, args),
                            usage: None,
                            tool_name: Some(name.to_string()),
                            tool_input_json: Some(args.to_string()),
                        });
                    }
                }
            }

            // Collect all content items and return as separate log entries
            if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
                let mut text_result: Option<String> = None;
                let mut think_result: Option<String> = None;

                for item in content {
                    match item.get("type").and_then(|v| v.as_str()) {
                        Some("text") => {
                            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                text_result = Some(text.to_string());
                            }
                        }
                        Some("think") => {
                            if let Some(think) = item.get("think").and_then(|v| v.as_str()) {
                                think_result = Some(think.to_string());
                            }
                        }
                        _ => {}
                    }
                }

                // Return text first if present (it's the final answer)
                if let Some(text) = text_result {
                    return Some(ParsedLogEntry {
                        timestamp: utc_timestamp(),
                        log_type: "text".to_string(),
                        content: text,
                        usage: None,
                tool_name: None,
                tool_input_json: None,
                    });
                }
                // Fall back to thinking
                if let Some(think) = think_result {
                    return Some(ParsedLogEntry {
                        timestamp: utc_timestamp(),
                        log_type: "thinking".to_string(),
                        content: think,
                        usage: None,
                tool_name: None,
                tool_input_json: None,
                    });
                }
            }
            return None;
        }

        // Tool result
        if role == "tool" {
            if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
                for item in content {
                    if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                            return Some(ParsedLogEntry {
                                timestamp: utc_timestamp(),
                                log_type: "tool_result".to_string(),
                                content: text.to_string(),
                                usage: None,
                tool_name: None,
                tool_input_json: None,
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

        // Classify stderr content by its nature
        let log_type = if trimmed.starts_with("[tool-streaming") {
            "tool".to_string()
        } else if trimmed.contains("error") || trimmed.contains("Error") || trimmed.contains("ERROR") || trimmed.contains("failed") || trimmed.contains("Failed") {
            "stderr".to_string()
        } else {
            "info".to_string()
        };

        Some(ParsedLogEntry {
            timestamp: utc_timestamp(),
            log_type,
            content: trimmed.to_string(),
            usage: None,
                tool_name: None,
                tool_input_json: None,
        })
    }

    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String> {
        // Get the last text response as final result
        logs.iter()
            .rev()
            .find(|l| l.log_type == "text")
            .map(|l| l.content.clone())
    }

    fn get_usage(&self, _logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
        self.usage.lock().clone()
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
        let executor = KimiExecutor::new("kimi".to_string());
        let args = executor.command_args("do something");
        assert_eq!(args, vec!["--print", "--output-format", "stream-json", "-p", "do something"]);
    }

    #[test]
    fn test_command_args_with_session() {
        let executor = KimiExecutor::new("kimi".to_string());
        let args = executor.command_args_with_session("continue task", Some("abc123"));
        assert_eq!(args, vec!["--print", "--output-format", "stream-json", "-p", "continue task", "-S", "abc123"]);
    }

    #[test]
    fn test_executor_type() {
        let executor = KimiExecutor::new("kimi".to_string());
        assert_eq!(executor.executor_type(), ExecutorType::Kimi);
    }

    #[test]
    fn test_parse_output_line_assistant_text() {
        let executor = KimiExecutor::new("kimi".to_string());
        let json = r#"{"role":"assistant","content":[{"type":"text","text":"Hello world"}]}"#;
        let entry = executor.parse_output_line(json).unwrap();
        assert_eq!(entry.log_type, "text");
        assert_eq!(entry.content, "Hello world");
    }

    #[test]
    fn test_parse_output_line_tool_call_request() {
        let executor = KimiExecutor::new("kimi".to_string());
        let json = r#"{"role":"assistant","content":[],"tool_calls":[{"type":"function","id":"call_1","function":{"name":"Shell","arguments":"{\"command\":\"date\"}"}}]}"#;
        let entry = executor.parse_output_line(json).unwrap();
        assert_eq!(entry.log_type, "tool_call");
        assert!(entry.content.contains("Shell"));
    }

    #[test]
    fn test_parse_output_line_tool_result() {
        let executor = KimiExecutor::new("kimi".to_string());
        let json = r#"{"role":"tool","content":[{"type":"text","text":"Tue Apr 28 07:59:16 PDT 2026"}]}"#;
        let entry = executor.parse_output_line(json).unwrap();
        assert_eq!(entry.log_type, "tool_result");
        assert_eq!(entry.content, "Tue Apr 28 07:59:16 PDT 2026");
    }

    #[test]
    fn test_parse_output_line_skip_resume() {
        let executor = KimiExecutor::new("kimi".to_string());
        let line = "To resume this session: kimi -r abc123";
        assert!(executor.parse_output_line(line).is_none());
    }
}
