use std::env;
use std::sync::Arc;
use parking_lot::Mutex;
use serde::Deserialize;

use super::{get_timestamp, CodeExecutor, ExecutorType, ParsedLogEntry, ExecutionUsage};

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
                        *self.model.lock() = Some(m.clone());
                    }
                    Some(ParsedLogEntry {
                        timestamp: get_timestamp(),
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
                            timestamp: get_timestamp(),
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
                            timestamp: get_timestamp(),
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
                        timestamp: get_timestamp(),
                        log_type: if is_error { "error".to_string() } else { "result".to_string() },
                        content: format!("{}{}", err_str, result_str),
                        usage,
                    })
                }
            };
        }

        // Fallback: treat as raw text
        Some(ParsedLogEntry {
            timestamp: get_timestamp(),
            log_type: "text".to_string(),
            content: line.to_string(),
            usage: None,
        })
    }

    fn check_success(&self, exit_code: i32) -> bool {
        exit_code == 0
    }

    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String> {
        // Claude Code 的结果在 result 类型日志中
        logs.iter()
            .rev()
            .find(|l| l.log_type == "result" || l.log_type == "text")
            .map(|l| l.content.clone())
    }

    fn get_usage(&self, logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
        // Usage is now captured directly in the ParsedLogEntry.usage field
        logs.iter().rev().find(|l| l.log_type == "result")?.usage.clone()
    }

    fn get_model(&self) -> Option<String> {
        self.model.lock().clone()
    }
}
