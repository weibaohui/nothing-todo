use serde::Deserialize;
use std::env;
use std::sync::Arc;
use parking_lot::Mutex;

use super::{get_timestamp, CodeExecutor, ExecutorType, ParsedLogEntry, ExecutionUsage};

pub struct OpencodeExecutor {
    path: String,
    model: Arc<Mutex<Option<String>>>,
    usage: Arc<Mutex<Option<ExecutionUsage>>>,
}

fn strip_think_tags(content: &str) -> String {
    use regex::Regex;
    let re = Regex::new(r"<think>[\s\S]*?</think>").unwrap();
    re.replace_all(content, "").trim().to_string()
}

impl OpencodeExecutor {
    pub fn new() -> Self {
        let path = env::var("OPENCODE_PATH")
            .unwrap_or_else(|_| "opencode".to_string());
        Self {
            path,
            model: Arc::new(Mutex::new(None)),
            usage: Arc::new(Mutex::new(None)),
        }
    }
}

impl Clone for OpencodeExecutor {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            model: self.model.clone(),
            usage: self.usage.clone(),
        }
    }
}

impl Default for OpencodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct OpencodeEvent {
    #[serde(rename = "type")]
    event_type: String,
    timestamp: u64,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    part: Option<OpencodePart>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpencodePart {
    #[serde(rename = "type")]
    part_type: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    message_id: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    state: Option<OpencodeToolState>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    tokens: Option<OpencodeTokens>,
    #[serde(default)]
    cost: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpencodeToolState {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    input: Option<OpencodeToolInput>,
    #[serde(default)]
    output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpencodeToolInput {
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpencodeTokens {
    total: u64,
    input: u64,
    output: u64,
    #[serde(default)]
    reasoning: u64,
    #[serde(default)]
    cache: OpencodeCacheTokens,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct OpencodeCacheTokens {
    #[serde(default)]
    read: u64,
    #[serde(default)]
    write: u64,
}

impl CodeExecutor for OpencodeExecutor {
    fn executor_type(&self) -> ExecutorType {
        ExecutorType::Opencode
    }

    fn executable_path(&self) -> &str {
        &self.path
    }

    fn command_args(&self, message: &str) -> Vec<String> {
        vec![
            "run".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "--dangerously-skip-permissions".to_string(),
            message.to_string(),
        ]
    }

    fn parse_output_line(&self, line: &str) -> Option<ParsedLogEntry> {
        let event: OpencodeEvent = serde_json::from_str(line).ok()?;

        let timestamp = chrono::DateTime::from_timestamp_millis(event.timestamp as i64)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
            .unwrap_or_else(get_timestamp);

        match event.event_type.as_str() {
            "step_start" => {
                Some(ParsedLogEntry {
                    timestamp,
                    log_type: "step_start".to_string(),
                    content: "Step started".to_string(),
                    usage: None,
                })
            }
            "tool_use" => {
                let part = event.part?;
                let tool = part.tool.unwrap_or_default();
                let status = part.state.as_ref().and_then(|s| s.status.clone()).unwrap_or_default();
                let description = part.state.as_ref().and_then(|s| s.input.as_ref().and_then(|i| i.description.clone())).unwrap_or_default();

                let content = if tool == "bash" {
                    if let Some(output) = &part.state.as_ref().and_then(|s| s.output.clone()) {
                        format!("[{}] {}: {}", status, description, output)
                    } else {
                        format!("[{}] {}", status, description)
                    }
                } else {
                    format!("[{}] Tool: {} - {}", status, tool, description)
                };

                Some(ParsedLogEntry {
                    timestamp,
                    log_type: "tool".to_string(),
                    content,
                    usage: None,
                })
            }
            "text" => {
                let part = event.part?;
                let text = part.text.unwrap_or_default();
                if text.is_empty() {
                    return None;
                }
                Some(ParsedLogEntry {
                    timestamp,
                    log_type: "text".to_string(),
                    content: text,
                    usage: None,
                })
            }
            "step_finish" => {
                // Store usage info if available
                if let Some(part) = &event.part {
                    if let Some(tokens) = &part.tokens {
                        let usage = ExecutionUsage {
                            input_tokens: tokens.input,
                            output_tokens: tokens.output,
                            cache_read_input_tokens: if tokens.cache.read > 0 { Some(tokens.cache.read) } else { None },
                            cache_creation_input_tokens: if tokens.cache.write > 0 { Some(tokens.cache.write) } else { None },
                            total_cost_usd: part.cost,
                            duration_ms: None,
                        };
                        *self.usage.lock() = Some(usage);
                    }
                }
                Some(ParsedLogEntry {
                    timestamp,
                    log_type: "step_finish".to_string(),
                    content: "Step finished".to_string(),
                    usage: None,
                })
            }
            _ => None,
        }
    }

    fn check_success(&self, exit_code: i32) -> bool {
        exit_code == 0
    }

    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String> {
        // 查找最后的 text 类型日志作为结果
        let text_result = logs.iter()
            .rev()
            .find(|l| l.log_type == "text")
            .map(|l| strip_think_tags(&l.content));

        // 如果没有 text，尝试 stderr
        let fallback = logs.iter()
            .rev()
            .find(|l| l.log_type == "stderr")
            .map(|l| l.content.clone());

        text_result.or(fallback)
    }

    fn get_usage(&self, _logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
        self.usage.lock().clone()
    }

    fn get_model(&self) -> Option<String> {
        self.model.lock().clone()
    }
}
