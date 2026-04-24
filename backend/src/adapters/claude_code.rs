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
                        })
                    }
                }
                ClaudeMessage::Result { result, is_error, duration_ms, total_cost_usd, usage, .. } => {
                    let err_str = if is_error { "[error] " } else { "" };
                    let result_str = result.unwrap_or_default();
                    let cost_str = total_cost_usd.map(|c| format!(" (${:.6})", c)).unwrap_or_default();
                    let duration_str = duration_ms.map(|d| format!(" [{}ms]", d)).unwrap_or_default();

                    // Append usage info as hidden comment for extraction later
                    let usage_str = if let Some(u) = usage {
                        format!(" /*usage:{}:{}:{}:{}*/",
                            u.input_tokens, u.output_tokens,
                            u.cache_read_input_tokens.unwrap_or(0),
                            u.cache_creation_input_tokens.unwrap_or(0))
                    } else {
                        String::new()
                    };

                    Some(ParsedLogEntry {
                        timestamp: get_timestamp(),
                        log_type: if is_error { "error".to_string() } else { "result".to_string() },
                        content: format!("{}{}{}{}{}", err_str, result_str, cost_str, duration_str, usage_str),
                    })
                }
            };
        }

        // Fallback: treat as raw text
        Some(ParsedLogEntry {
            timestamp: get_timestamp(),
            log_type: "text".to_string(),
            content: line.to_string(),
        })
    }

    fn check_success(&self, exit_code: i32) -> bool {
        exit_code == 0
    }

    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String> {
        use regex::Regex;

        // Claude Code 的结果在 result 类型日志中
        let content = logs.iter()
            .rev()
            .find(|l| l.log_type == "result" || l.log_type == "text")
            .map(|l| l.content.clone())?;

        // 去掉 /*usage:...*/ 注释
        let usage_re = Regex::new(r"/\*usage:[0-9]+:[0-9]+:[0-9]+:[0-9]+\*/").ok()?;
        Some(usage_re.replace_all(&content, "").to_string())
    }

    fn get_usage(&self, logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
        use regex::Regex;

        // Find the result log which contains cost, duration and usage info
        let result_log = logs.iter().rev().find(|l| l.log_type == "result")?;
        let content = &result_log.content;

        // Extract cost: " ($0.026710)"
        let cost_re = Regex::new(r"\(\$([0-9.]+)\)").ok()?;
        let cost = cost_re.captures(content)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok());

        // Extract duration: " [12824ms]"
        let duration_re = Regex::new(r"\[([0-9]+)ms\]").ok()?;
        let duration = duration_re.captures(content)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<u64>().ok());

        // Extract usage info from the same content: "/*usage:123:456:789:0*/"
        let usage_re = Regex::new(r"/\*usage:([0-9]+):([0-9]+):([0-9]+):([0-9]+)\*/").ok()?;
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut cache_read: Option<u64> = None;
        let mut cache_creation: Option<u64> = None;

        if let Some(caps) = usage_re.captures(content) {
            input_tokens = caps.get(1).and_then(|m| m.as_str().parse::<u64>().ok()).unwrap_or(0);
            output_tokens = caps.get(2).and_then(|m| m.as_str().parse::<u64>().ok()).unwrap_or(0);
            let cr = caps.get(3).and_then(|m| m.as_str().parse::<u64>().ok());
            let cc = caps.get(4).and_then(|m| m.as_str().parse::<u64>().ok());
            if cr.unwrap_or(0) > 0 {
                cache_read = cr;
            }
            if cc.unwrap_or(0) > 0 {
                cache_creation = cc;
            }
        }

        Some(ExecutionUsage {
            input_tokens,
            output_tokens,
            cache_read_input_tokens: cache_read,
            cache_creation_input_tokens: cache_creation,
            total_cost_usd: cost,
            duration_ms: duration,
        })
    }

    fn get_model(&self) -> Option<String> {
        self.model.lock().clone()
    }
}
