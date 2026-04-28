use serde::Deserialize;
use std::env;
use std::sync::{Arc, Mutex};

use super::{CodeExecutor, ExecutorType, ParsedLogEntry, ExecutionUsage};
use crate::models::utc_timestamp;

pub struct OpencodeExecutor {
    path: String,
    model: Arc<Mutex<Option<String>>>,
    usage: Arc<Mutex<Option<ExecutionUsage>>>,
    has_successful_finish: Arc<Mutex<bool>>,
}

impl OpencodeExecutor {
    pub fn new() -> Self {
        let path = env::var("OPENCODE_PATH")
            .unwrap_or_else(|_| "opencode".to_string());
        Self {
            path,
            model: Arc::new(Mutex::new(None)),
            usage: Arc::new(Mutex::new(None)),
            has_successful_finish: Arc::new(Mutex::new(false)),
        }
    }
}

impl Clone for OpencodeExecutor {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            model: self.model.clone(),
            usage: self.usage.clone(),
            has_successful_finish: self.has_successful_finish.clone(),
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

    fn command_args_with_session(&self, message: &str, session_id: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        if let Some(sid) = session_id {
            args.push("--session".to_string());
            args.push(sid.to_string());
        }
        args.push("--dangerously-skip-permissions".to_string());
        args.push(message.to_string());
        args
    }

    fn parse_output_line(&self, line: &str) -> Option<ParsedLogEntry> {
        let event: OpencodeEvent = serde_json::from_str(line).ok()?;

        let timestamp = chrono::DateTime::from_timestamp_millis(event.timestamp as i64)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
            .unwrap_or_else(utc_timestamp);

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
                // Mark as successfully finished — opencode returns non-zero exit code
                // even on successful execution, so we track success via the event stream.
                *self.has_successful_finish.lock().unwrap() = true;

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
        // 查找最后的 text 类型日志作为结果
        let text_result = logs.iter()
            .rev()
            .find(|l| l.log_type == "text")
            .map(|l| super::strip_think_tags(&l.content));

        // 如果没有 text，尝试 stderr
        let fallback = logs.iter()
            .rev()
            .find(|l| l.log_type == "stderr")
            .map(|l| l.content.clone());

        text_result.or(fallback)
    }

    fn get_usage(&self, _logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
        self.usage.lock().unwrap().clone()
    }

    fn get_model(&self) -> Option<String> {
        self.model.lock().unwrap().clone()
    }

    fn check_success(&self, exit_code: i32) -> bool {
        if exit_code == 0 {
            return true;
        }
        // opencode returns non-zero exit codes (e.g. 144) even on successful execution.
        // Trust the presence of a step_finish event in the output stream.
        *self.has_successful_finish.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ParsedLogEntry;

    #[test]
    fn test_parse_output_line_step_start() {
        let executor = OpencodeExecutor::new();
        let line = r#"{"type":"step_start","timestamp":1700000000000}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "step_start");
        assert_eq!(entry.content, "Step started");
    }

    #[test]
    fn test_parse_output_line_tool_use_bash() {
        let executor = OpencodeExecutor::new();
        let line = r#"{"type":"tool_use","timestamp":1700000000000,"part":{"type":"tool_use","tool":"bash","state":{"status":"success","input":{"description":"list files"},"output":"file.txt"}}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "tool");
        assert!(entry.content.contains("success"), "content should contain status: {}", entry.content);
        assert!(entry.content.contains("list files"), "content should contain description: {}", entry.content);
        assert!(entry.content.contains("file.txt"), "content should contain output: {}", entry.content);
    }

    #[test]
    fn test_parse_output_line_text() {
        let executor = OpencodeExecutor::new();
        let line = r#"{"type":"text","timestamp":1700000000000,"part":{"type":"text","text":"hello world"}}"#;
        let entry = executor.parse_output_line(line).unwrap();
        assert_eq!(entry.log_type, "text");
        assert_eq!(entry.content, "hello world");
    }

    #[test]
    fn test_parse_output_line_step_finish_stores_usage() {
        let executor = OpencodeExecutor::new();
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
        let executor = OpencodeExecutor::new();
        let line = r#"{"type":"unknown","timestamp":1700000000000}"#;
        assert!(executor.parse_output_line(line).is_none());
    }

    #[test]
    fn test_parse_output_line_invalid_json() {
        let executor = OpencodeExecutor::new();
        let line = "not json";
        assert!(executor.parse_output_line(line).is_none());
    }

    #[test]
    fn test_parse_output_line_empty_text() {
        let executor = OpencodeExecutor::new();
        let line = r#"{"type":"text","timestamp":1700000000000,"part":{"type":"text","text":""}}"#;
        assert!(executor.parse_output_line(line).is_none());
    }

    #[test]
    fn test_get_final_result_with_text() {
        let executor = OpencodeExecutor::new();
        let logs = vec![
            ParsedLogEntry::new("text", "  hello world  "),
        ];
        assert_eq!(executor.get_final_result(&logs), Some("hello world".to_string()));
    }

    #[test]
    fn test_get_final_result_fallback_to_stderr() {
        let executor = OpencodeExecutor::new();
        let logs = vec![
            ParsedLogEntry::new("stderr", "error output"),
        ];
        assert_eq!(executor.get_final_result(&logs), Some("error output".to_string()));
    }

    #[test]
    fn test_get_final_result_empty_logs() {
        let executor = OpencodeExecutor::new();
        let logs: Vec<ParsedLogEntry> = vec![];
        assert!(executor.get_final_result(&logs).is_none());
    }

    #[test]
    fn test_get_usage_before_step_finish() {
        let executor = OpencodeExecutor::new();
        assert!(executor.get_usage(&[]).is_none());
    }

    #[test]
    fn test_get_model_always_none() {
        let executor = OpencodeExecutor::new();
        assert!(executor.get_model().is_none());
    }

    #[test]
    fn test_check_success_exit_code_zero() {
        let executor = OpencodeExecutor::new();
        assert!(executor.check_success(0));
    }

    #[test]
    fn test_check_success_non_zero_without_step_finish() {
        let executor = OpencodeExecutor::new();
        assert!(!executor.check_success(144));
        assert!(!executor.check_success(1));
    }

    #[test]
    fn test_check_success_non_zero_with_step_finish() {
        let executor = OpencodeExecutor::new();
        let line = r#"{"type":"step_finish","timestamp":1700000000000,"part":{"type":"step_finish","tokens":{"total":100,"input":50,"output":50,"cache":{"read":10,"write":5}},"cost":0.001}}"#;
        let _ = executor.parse_output_line(line);
        assert!(executor.check_success(144), "should succeed when step_finish was parsed even with non-zero exit code");
    }
}

