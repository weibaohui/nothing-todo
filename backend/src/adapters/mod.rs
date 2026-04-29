use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::models::{ExecutorType, ParsedLogEntry, ExecutionUsage, TodoItem};

/// Parse executor string (with aliases) into `ExecutorType`.
/// Returns `None` for unrecognized names.
pub fn parse_executor_type(executor: &str) -> Option<ExecutorType> {
    match executor.trim().to_lowercase().as_str() {
        "claudecode" | "claude" => Some(ExecutorType::Claudecode),
        "codebuddy" | "cbc" => Some(ExecutorType::Codebuddy),
        "opencode" => Some(ExecutorType::Opencode),
        "atomcode" | "atom" => Some(ExecutorType::Atomcode),
        "hermes" => Some(ExecutorType::Hermes),
        "kimi" => Some(ExecutorType::Kimi),
        "joinai" => Some(ExecutorType::Joinai),
        _ => None,
    }
}

/// Strip `<think>...</think>` tags from content.
pub fn strip_think_tags(content: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;
    static THINK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<think>[\s\S]*?</think>").unwrap()
    });
    THINK_RE.replace_all(content, "").trim().to_string()
}

/// Default `get_final_result` for executors that use text+stderr logs with think-tag stripping.
/// Returns the last "text" log (with think tags stripped), falling back to last "stderr" log.
pub fn default_final_result_with_think_stripping(logs: &[ParsedLogEntry]) -> Option<String> {
    let text_result = logs.iter()
        .rev()
        .find(|l| l.log_type == "text")
        .map(|l| strip_think_tags(&l.content));

    let fallback = logs.iter()
        .rev()
        .find(|l| l.log_type == "stderr")
        .map(|l| l.content.clone());

    text_result.or(fallback)
}

/// Extract usage from the last "result" log entry (used by claude_code, codebuddy).
pub fn get_usage_from_logs(logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> {
    logs.iter().rev().find(|l| l.log_type == "result")?.usage.clone()
}

pub mod joinai;
pub mod claude_protocol;
pub mod agent_event;
pub mod claude_code;
pub mod codebuddy;
pub mod opencode;
pub mod atomcode;
pub mod hermes;
pub mod kimi;

#[async_trait]
pub trait CodeExecutor: Send + Sync {
    /// 返回执行器类型
    fn executor_type(&self) -> ExecutorType;

    /// 返回可执行文件路径
    fn executable_path(&self) -> &str;

    /// 返回命令参数
    fn command_args(&self, message: &str) -> Vec<String>;

    /// 返回带 session 的命令参数（默认实现忽略 session）
    fn command_args_with_session(&self, message: &str, _session_id: Option<&str>) -> Vec<String> {
        self.command_args(message)
    }

    /// 解析输出行，返回解析后的日志条目
    fn parse_output_line(&self, line: &str) -> Option<ParsedLogEntry>;

    /// 解析 stderr 行，返回解析后的日志条目。返回 None 表示作为普通 stderr 处理。
    fn parse_stderr_line(&self, _line: &str) -> Option<ParsedLogEntry> {
        None
    }

    /// 是否解析成功（检查退出码）
    fn check_success(&self, exit_code: i32) -> bool {
        exit_code == 0
    }

    /// 从日志列表中提取最终结果
    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String> {
        logs.iter()
            .rev()
            .find(|l| l.log_type == "result" || l.log_type == "text")
            .map(|l| l.content.clone())
    }

    /// 从日志列表中提取 usage 信息
    fn get_usage(&self, logs: &[ParsedLogEntry]) -> Option<ExecutionUsage>;
    fn get_model(&self) -> Option<String>;

    /// 执行完成后从外部数据源提取 todo 进度（用于无法从 stdout 获取工具调用的执行器）
    fn post_execution_todo_progress(&self) -> Option<Vec<TodoItem>> {
        None
    }
}

/// 代码执行器注册表
pub struct ExecutorRegistry {
    executors: Arc<RwLock<HashMap<ExecutorType, Arc<dyn CodeExecutor>>>>,
}

impl ExecutorRegistry {
    pub fn new() -> Self {
        Self {
            executors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register<E: CodeExecutor + 'static>(&self, executor: E) {
        let executor_type = executor.executor_type();
        self.executors.write().unwrap().insert(executor_type, Arc::new(executor));
    }

    pub fn get(&self, executor_type: ExecutorType) -> Option<Arc<dyn CodeExecutor>> {
        self.executors.read().unwrap().get(&executor_type).cloned()
    }

    pub fn get_default(&self) -> Option<Arc<dyn CodeExecutor>> {
        self.get(ExecutorType::Claudecode)
    }

    pub fn list_executors(&self) -> Vec<ExecutorType> {
        self.executors.read().unwrap().keys().copied().collect()
    }
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ParsedLogEntry, ExecutionUsage};

    #[test]
    fn test_parse_executor_type_claudecode() {
        assert_eq!(parse_executor_type("claudecode"), Some(ExecutorType::Claudecode));
        assert_eq!(parse_executor_type("claude"), Some(ExecutorType::Claudecode));
    }

    #[test]
    fn test_parse_executor_type_codebuddy() {
        assert_eq!(parse_executor_type("codebuddy"), Some(ExecutorType::Codebuddy));
        assert_eq!(parse_executor_type("cbc"), Some(ExecutorType::Codebuddy));
    }

    #[test]
    fn test_parse_executor_type_opencode() {
        assert_eq!(parse_executor_type("opencode"), Some(ExecutorType::Opencode));
    }

    #[test]
    fn test_parse_executor_type_atomcode() {
        assert_eq!(parse_executor_type("atomcode"), Some(ExecutorType::Atomcode));
        assert_eq!(parse_executor_type("atom"), Some(ExecutorType::Atomcode));
        assert_eq!(parse_executor_type("ATOMCODE"), Some(ExecutorType::Atomcode));
    }

    #[test]
    fn test_parse_executor_type_joinai() {
        assert_eq!(parse_executor_type("joinai"), Some(ExecutorType::Joinai));
    }

    #[test]
    fn test_parse_executor_type_unknown() {
        assert_eq!(parse_executor_type("unknown"), None);
        assert_eq!(parse_executor_type(""), None);
        assert_eq!(parse_executor_type("typo_executor"), None);
    }

    #[test]
    fn test_parse_executor_type_case_insensitive() {
        assert_eq!(parse_executor_type("Claude"), Some(ExecutorType::Claudecode));
        assert_eq!(parse_executor_type("CLAUDE"), Some(ExecutorType::Claudecode));
        assert_eq!(parse_executor_type("CodeBuddy"), Some(ExecutorType::Codebuddy));
    }

    #[test]
    fn test_parse_executor_type_trims_whitespace() {
        assert_eq!(parse_executor_type(" claude "), Some(ExecutorType::Claudecode));
        assert_eq!(parse_executor_type("  opencode"), Some(ExecutorType::Opencode));
        assert_eq!(parse_executor_type("kimi  "), Some(ExecutorType::Kimi));
    }

    #[test]
    fn test_strip_think_tags_basic() {
        assert_eq!(strip_think_tags("<think>x</think>hello"), "hello");
    }

    #[test]
    fn test_strip_think_tags_multiline() {
        let input = "<think>\nline1\nline2\n</think>result";
        assert_eq!(strip_think_tags(input), "result");
    }

    #[test]
    fn test_strip_think_tags_no_tags() {
        assert_eq!(strip_think_tags("hello world"), "hello world");
    }

    #[test]
    fn test_strip_think_tags_multiple() {
        assert_eq!(strip_think_tags("<think>a</think><think>b</think>c"), "c");
    }

    #[test]
    fn test_executor_registry_new_empty() {
        let reg = ExecutorRegistry::new();
        assert!(reg.list_executors().is_empty());
    }

    #[test]
    fn test_executor_registry_register_and_get() {
        let reg = ExecutorRegistry::new();
        reg.register(joinai::JoinaiExecutor::new("joinai".to_string()));
        assert!(reg.get(ExecutorType::Joinai).is_some());
    }

    #[test]
    fn test_executor_registry_get_default() {
        let reg = ExecutorRegistry::new();
        reg.register(claude_code::ClaudeCodeExecutor::new("claude".to_string()));
        assert!(reg.get_default().is_some());
    }

    #[test]
    fn test_executor_registry_get_default_when_empty() {
        let reg = ExecutorRegistry::new();
        assert!(reg.get_default().is_none());
    }

    #[test]
    fn test_executor_registry_list_executors() {
        let reg = ExecutorRegistry::new();
        reg.register(joinai::JoinaiExecutor::new("joinai".to_string()));
        reg.register(claude_code::ClaudeCodeExecutor::new("claude".to_string()));
        let list = reg.list_executors();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&ExecutorType::Joinai));
        assert!(list.contains(&ExecutorType::Claudecode));
    }

    // 使用一个最小的 mock 实现来测试 trait 默认方法
    struct MockExecutor;

    #[async_trait]
    impl CodeExecutor for MockExecutor {
        fn executor_type(&self) -> ExecutorType { ExecutorType::Joinai }
        fn executable_path(&self) -> &str { "mock" }
        fn command_args(&self, _message: &str) -> Vec<String> { vec![] }
        fn parse_output_line(&self, _line: &str) -> Option<ParsedLogEntry> { None }
        fn get_usage(&self, _logs: &[ParsedLogEntry]) -> Option<ExecutionUsage> { None }
        fn get_model(&self) -> Option<String> { None }
    }

    #[test]
    fn test_code_executor_default_check_success() {
        let exec = MockExecutor;
        assert!(exec.check_success(0));
        assert!(!exec.check_success(1));
        assert!(!exec.check_success(-1));
    }

    #[test]
    fn test_code_executor_default_get_final_result() {
        let exec = MockExecutor;
        let logs = vec![
            ParsedLogEntry::new("info", "start"),
            ParsedLogEntry::new("text", "partial"),
            ParsedLogEntry::new("result", "final answer"),
        ];
        assert_eq!(exec.get_final_result(&logs), Some("final answer".to_string()));
    }

    #[test]
    fn test_code_executor_default_get_final_result_fallback_to_text() {
        let exec = MockExecutor;
        let logs = vec![
            ParsedLogEntry::new("info", "start"),
            ParsedLogEntry::new("text", "only text"),
        ];
        assert_eq!(exec.get_final_result(&logs), Some("only text".to_string()));
    }

    #[test]
    fn test_code_executor_default_get_final_result_no_match() {
        let exec = MockExecutor;
        let logs = vec![ParsedLogEntry::new("info", "start")];
        assert_eq!(exec.get_final_result(&logs), None);
    }
}

