use std::env;
use std::sync::{Arc, Mutex};
use serde::Deserialize;

use super::{CodeExecutor, ExecutorType, ParsedLogEntry, ExecutionUsage};
use crate::models::utc_timestamp;

pub struct ClaudeCodeExecutor {
    path: String,
    model: Arc<Mutex<Option<String>>>,
}

impl ClaudeCodeExecutor {
    pub fn new() -> Self {
        let path = env::var("CLAUDECODE_PATH")
            .unwrap_or_else(|_| "claude".to_string());
        Self { path, model: Arc::new(Mutex::new(None)) }
    }
}

impl Clone for ClaudeCodeExecutor {
    fn clone(&self) -> Self {
        Self { path: self.path.clone(), model: self.model.clone() }
    }
}

impl Default for ClaudeCodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ClaudeMessage {
    #[serde(rename = "system")]
    System {
        subtype: Option<String>,
        session_id: Option<String>,
        model: Option<String>,
    },
    #[serde(rename = "assistant")]
    Assistant {
        message: ClaudeMessageContent,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
        session_id: Option<String>,
        uuid: Option<String>,
    },
    #[serde(rename = "user")]
    User {
        message: ClaudeMessageContent,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
        session_id: Option<String>,
        uuid: Option<String>,
    },
    #[serde(rename = "result")]
    Result {
        subtype: Option<String>,
        is_error: bool,
        duration_ms: Option<u64>,
        result: Option<String>,
        total_cost_usd: Option<f64>,
        #[serde(default)]
        usage: Option<ClaudeUsage>,
        session_id: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct ClaudeMessageContent {
    id: Option<String>,
    #[serde(rename = "type")]
    content_type: Option<String>,
    role: Option<String>,
    #[serde(default)]
    content: Vec<ClaudeContentBlock>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ClaudeContentBlock {
    #[serde(rename = "thinking")]
    Thinking { thinking: Option<String> },
    #[serde(rename = "text")]
    Text { text: Option<String> },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: Option<String>,
        name: Option<String>,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: Option<String>,
        content: Option<String>,
        is_error: Option<bool>,
    },
    #[serde(rename = "redacted")]
    Redacted { redacted: Option<String> },
}

#[derive(Debug, Clone, Deserialize)]
struct ClaudeUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
}

impl CodeExecutor for ClaudeCodeExecutor {
    fn executor_type(&self) -> ExecutorType {
        ExecutorType::Claudecode
    }

    fn executable_path(&self) -> &str {
        &self.path
    }

    fn command_args(&self, message: &str) -> Vec<String> {
        vec![
            "--dangerously-skip-permissions".to_string(),
            "-p".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
            message.to_string(),
        ]
    }

    fn command_args_with_session(&self, message: &str, session_id: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "--dangerously-skip-permissions".to_string(),
            "-p".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
        ];
        if let Some(sid) = session_id {
            args.push("--session-id".to_string());
            args.push(sid.to_string());
        }
        args.push("--verbose".to_string());
        args.push(message.to_string());
        args
    }

    fn parse_output_line(&self, line: &str) -> Option<ParsedLogEntry> {
        if line.is_empty() {
            return None;
        }

        // Try to parse as Claude NDJSON message
        if let Ok(msg) = serde_json::from_str::<ClaudeMessage>(line) {
            return match msg {
                ClaudeMessage::System { subtype, session_id, model } => {
                    // Store model if found
                    if let Some(m) = model {
                        *self.model.lock().unwrap() = Some(m.clone());
                    }
                    Some(ParsedLogEntry {
                        timestamp: utc_timestamp(),
                        log_type: "system".to_string(),
                        content: format!("Session init: {:?}", session_id.or(subtype)),
                        usage: None,
                    })
                }
                ClaudeMessage::Assistant { message, .. } => {
                    let mut parts = Vec::new();
                    for block in message.content {
                        match block {
                            ClaudeContentBlock::Thinking { thinking } => {
                                if let Some(t) = thinking {
                                    parts.push(format!("[thinking] {}", t.chars().take(200).collect::<String>()));
                                }
                            }
                            ClaudeContentBlock::Text { text } => {
                                if let Some(t) = text {
                                    parts.push(t);
                                }
                            }
                            ClaudeContentBlock::ToolUse { name, input, .. } => {
                                let input_str = serde_json::to_string(&input).unwrap_or_default();
                                parts.push(format!("[tool] {}: {}", name.unwrap_or_default(), input_str.chars().take(100).collect::<String>()));
                            }
                            ClaudeContentBlock::ToolResult { content, is_error, .. } => {
                                let err_str = if is_error.unwrap_or(false) { "[error] " } else { "" };
                                parts.push(format!("{}{}", err_str, content.unwrap_or_default().chars().take(100).collect::<String>()));
                            }
                            ClaudeContentBlock::Redacted { redacted } => {
                                parts.push(format!("[redacted] {}", redacted.unwrap_or_default()));
                            }
                        }
                    }
                    if parts.is_empty() {
                        None
                    } else {
                        Some(ParsedLogEntry {
                            timestamp: utc_timestamp(),
                            log_type: "assistant".to_string(),
                            content: parts.join("\n"),
                            usage: None,
                        })
                    }
                }
                ClaudeMessage::User { message, .. } => {
                    let mut parts = Vec::new();
                    for block in message.content {
                        if let ClaudeContentBlock::ToolResult { content, is_error, .. } = block {
                            let err_str = if is_error.unwrap_or(false) { "[error] " } else { "" };
                            parts.push(format!("{}{}", err_str, content.unwrap_or_default()));
                        }
                    }
                    if parts.is_empty() {
                        None
                    } else {
                        Some(ParsedLogEntry {
                            timestamp: utc_timestamp(),
                            log_type: "user".to_string(),
                            content: parts.join("\n"),
                            usage: None,
                        })
                    }
                }
                ClaudeMessage::Result { result, is_error, duration_ms, total_cost_usd, usage, .. } => {
                    let err_str = if is_error { "[error] " } else { "" };
                    let result_str = result.unwrap_or_default();

                    // Build usage from Result fields
                    let usage = usage.map(|u| crate::models::ExecutionUsage {
                        input_tokens: u.input_tokens,
                        output_tokens: u.output_tokens,
                        cache_read_input_tokens: u.cache_read_input_tokens,
                        cache_creation_input_tokens: u.cache_creation_input_tokens,
                        total_cost_usd,
                        duration_ms,
                    });

                    Some(ParsedLogEntry {
                        timestamp: utc_timestamp(),
                        log_type: if is_error { "error".to_string() } else { "result".to_string() },
                        content: format!("{}{}", err_str, result_str),
                        usage,
                    })
                }
            };
        }

        // Fallback: treat as raw text
        Some(ParsedLogEntry {
            timestamp: utc_timestamp(),
            log_type: "text".to_string(),
            content: line.to_string(),
            usage: None,
        })
    }

    fn get_usage(&self, logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
        // Usage is now captured directly in the ParsedLogEntry.usage field
        logs.iter().rev().find(|l| l.log_type == "result")?.usage.clone()
    }

    fn get_model(&self) -> Option<String> {
        self.model.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ParsedLogEntry;

    #[test]
    fn test_parse_output_line_system() {
        let executor = ClaudeCodeExecutor::new();
        let line = r#"{"type":"system","model":"claude-3-5-sonnet"}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "system");
        assert!(entry.content.contains("Session init"));
        assert_eq!(executor.get_model(), Some("claude-3-5-sonnet".to_string()));
    }

    #[test]
    fn test_parse_output_line_assistant_text() {
        let executor = ClaudeCodeExecutor::new();
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hello"}]}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "assistant");
        assert_eq!(entry.content, "hello");
    }

    #[test]
    fn test_parse_output_line_assistant_thinking() {
        let executor = ClaudeCodeExecutor::new();
        let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"thinking..."}]}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "assistant");
        assert!(entry.content.starts_with("[thinking]"));
        assert!(entry.content.contains("thinking..."));
    }

    #[test]
    fn test_parse_output_line_user_tool_result() {
        let executor = ClaudeCodeExecutor::new();
        let line = r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"result","is_error":false}]}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "user");
        assert_eq!(entry.content, "result");
    }

    #[test]
    fn test_parse_output_line_result_success() {
        let executor = ClaudeCodeExecutor::new();
        let line = r#"{"type":"result","result":"final","is_error":false,"duration_ms":100,"total_cost_usd":0.001,"usage":{"input_tokens":10,"output_tokens":20}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "result");
        assert_eq!(entry.content, "final");
        assert!(entry.usage.is_some());
        let usage = entry.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.duration_ms, Some(100));
        assert_eq!(usage.total_cost_usd, Some(0.001));
    }

    #[test]
    fn test_parse_output_line_result_error() {
        let executor = ClaudeCodeExecutor::new();
        let line = r#"{"type":"result","result":"error","is_error":true}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "error");
        assert_eq!(entry.content, "[error] error");
    }

    #[test]
    fn test_parse_output_line_empty_line() {
        let executor = ClaudeCodeExecutor::new();
        let line = "";
        assert!(executor.parse_output_line(line).is_none());
    }

    #[test]
    fn test_parse_output_line_raw_text_fallback() {
        let executor = ClaudeCodeExecutor::new();
        let line = "just text";
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "text");
        assert_eq!(entry.content, "just text");
    }

    #[test]
    fn test_get_usage_after_result() {
        let executor = ClaudeCodeExecutor::new();
        let logs = vec![
            ParsedLogEntry {
                timestamp: utc_timestamp(),
                log_type: "result".to_string(),
                content: "final".to_string(),
                usage: Some(ExecutionUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                    total_cost_usd: Some(0.001),
                    duration_ms: Some(100),
                }),
            },
        ];
        let usage = executor.get_usage(&logs).unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
    }

    #[test]
    fn test_get_usage_no_result() {
        let executor = ClaudeCodeExecutor::new();
        let logs: Vec<ParsedLogEntry> = vec![];
        assert!(executor.get_usage(&logs).is_none());
    }

    #[test]
    fn test_get_model_before_system() {
        let executor = ClaudeCodeExecutor::new();
        assert!(executor.get_model().is_none());
    }
}
