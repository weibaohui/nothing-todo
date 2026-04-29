use serde::Deserialize;
use std::env;
use std::sync::{Arc, Mutex};

use super::{CodeExecutor, ExecutorType, ParsedLogEntry, ExecutionUsage};
use crate::models::utc_timestamp;

pub struct JoinaiExecutor {
    path: String,
    usage: Arc<Mutex<Option<ExecutionUsage>>>,
}

impl JoinaiExecutor {
    pub fn new() -> Self {
        let path = env::var("JOINAI_PATH")
            .unwrap_or_else(|_| "joinai".to_string());
        Self { path, usage: Arc::new(Mutex::new(None)) }
    }
}

impl Clone for JoinaiExecutor {
    fn clone(&self) -> Self {
        Self { path: self.path.clone(), usage: self.usage.clone() }
    }
}

impl Default for JoinaiExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct JoinaiEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    part: Option<JoinaiPart>,
    timestamp: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct JoinaiPart {
    #[serde(rename = "type")]
    part_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    state: Option<JoinaiToolState>,
    message_id: Option<String>,
    session_id: Option<String>,
    #[serde(default)]
    tokens: Option<JoinaiTokens>,
    #[serde(default)]
    cost: Option<f64>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct JoinaiToolState {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    input: Option<JoinaiToolInput>,
    #[serde(default)]
    output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct JoinaiToolInput {
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct JoinaiTokens {
    total: u64,
    input: u64,
    output: u64,
    #[serde(default)]
    reasoning: u64,
    #[serde(default)]
    cache: JoinaiCacheTokens,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct JoinaiCacheTokens {
    #[serde(default)]
    read: u64,
    #[serde(default)]
    write: u64,
}

impl CodeExecutor for JoinaiExecutor {
    fn executor_type(&self) -> ExecutorType {
        ExecutorType::Joinai
    }

    fn executable_path(&self) -> &str {
        &self.path
    }

    fn command_args(&self, message: &str) -> Vec<String> {
        vec![
            "run".to_string(),
            "--format".to_string(),
            "json".to_string(),
            message.to_string(),
        ]
    }

    fn parse_output_line(&self, line: &str) -> Option<ParsedLogEntry> {
        let event: JoinaiEvent = serde_json::from_str(line).ok()?;

        let timestamp = event.timestamp
            .map(|ts| {
                let secs = ts / 1000;
                let millis = ts % 1000;
                format!("{}.{:03}", secs, millis)
            })
            .unwrap_or_else(utc_timestamp);

        match event.event_type.as_str() {
            "step_start" => {
                Some(ParsedLogEntry {
                    timestamp,
                    log_type: "step_start".to_string(),
                    content: format!("Step started"),
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
                let text = event.part?.text.unwrap_or_default();
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
                        *self.usage.lock().unwrap() = Some(usage);
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

    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String> {
        super::default_final_result_with_think_stripping(logs)
    }

    fn get_usage(&self, _logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
        self.usage.lock().unwrap().clone()
    }

    fn get_model(&self) -> Option<String> {
        // Joinai doesn't provide model info
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ParsedLogEntry;

    #[test]
    fn test_parse_output_line_step_start() {
        let executor = JoinaiExecutor::new();
        let line = r#"{"type":"step_start","timestamp":1700000000000}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "step_start");
        assert_eq!(entry.content, "Step started");
    }

    #[test]
    fn test_parse_output_line_tool_use_bash() {
        let executor = JoinaiExecutor::new();
        let line = r#"{"type":"tool_use","timestamp":1700000000000,"part":{"type":"tool_use","tool":"bash","state":{"status":"success","input":{"description":"list files"},"output":"file.txt"}}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "tool");
        assert!(entry.content.contains("success"), "content should contain status: {}", entry.content);
        assert!(entry.content.contains("list files"), "content should contain description: {}", entry.content);
        assert!(entry.content.contains("file.txt"), "content should contain output: {}", entry.content);
    }

    #[test]
    fn test_parse_output_line_text() {
        let executor = JoinaiExecutor::new();
        let line = r#"{"type":"text","timestamp":1700000000000,"part":{"type":"text","text":"hello world"}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "text");
        assert_eq!(entry.content, "hello world");
    }

    #[test]
    fn test_parse_output_line_step_finish_stores_usage() {
        let executor = JoinaiExecutor::new();
        let line = r#"{"type":"step_finish","timestamp":1700000000000,"part":{"type":"step_finish","tokens":{"total":100,"input":50,"output":50,"cache":{"read":10,"write":5}},"cost":0.001}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "step_finish");
        assert_eq!(entry.content, "Step finished");

        let usage = executor.get_usage(&[]).unwrap();
        assert_eq!(usage.input_tokens, 50);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.cache_read_input_tokens, Some(10));
        assert_eq!(usage.cache_creation_input_tokens, Some(5));
        assert_eq!(usage.total_cost_usd, Some(0.001));
    }

    #[test]
    fn test_parse_output_line_unknown_type() {
        let executor = JoinaiExecutor::new();
        let line = r#"{"type":"unknown","timestamp":1700000000000}"#;
        assert!(executor.parse_output_line(line).is_none());
    }

    #[test]
    fn test_parse_output_line_invalid_json() {
        let executor = JoinaiExecutor::new();
        let line = "not json";
        assert!(executor.parse_output_line(line).is_none());
    }

    #[test]
    fn test_parse_output_line_empty_text() {
        let executor = JoinaiExecutor::new();
        let line = r#"{"type":"text","timestamp":1700000000000,"part":{"type":"text","text":""}}"#;
        assert!(executor.parse_output_line(line).is_none());
    }

    #[test]
    fn test_get_final_result_with_text() {
        let executor = JoinaiExecutor::new();
        let logs = vec![
            ParsedLogEntry::new("text", "  hello world  "),
        ];
        assert_eq!(executor.get_final_result(&logs), Some("hello world".to_string()));
    }

    #[test]
    fn test_get_final_result_fallback_to_stderr() {
        let executor = JoinaiExecutor::new();
        let logs = vec![
            ParsedLogEntry::new("stderr", "error output"),
        ];
        assert_eq!(executor.get_final_result(&logs), Some("error output".to_string()));
    }

    #[test]
    fn test_get_final_result_empty_logs() {
        let executor = JoinaiExecutor::new();
        let logs: Vec<ParsedLogEntry> = vec![];
        assert!(executor.get_final_result(&logs).is_none());
    }

    #[test]
    fn test_get_usage_before_step_finish() {
        let executor = JoinaiExecutor::new();
        assert!(executor.get_usage(&[]).is_none());
    }

    #[test]
    fn test_get_model_always_none() {
        let executor = JoinaiExecutor::new();
        assert!(executor.get_model().is_none());
    }
}


