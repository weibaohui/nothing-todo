use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;

use crate::models::{ExecutorType, ParsedLogEntry, ExecutionUsage};

pub mod joinai;
pub mod claude_code;
pub mod codebuddy;
pub mod opencode;

#[async_trait]
pub trait CodeExecutor: Send + Sync {
    /// 返回执行器类型
    fn executor_type(&self) -> ExecutorType;

    /// 返回可执行文件路径
    fn executable_path(&self) -> &str;

    /// 返回默认命令参数
    fn command_args(&self, message: &str) -> Vec<String>;

    /// 解析输出行，返回解析后的日志条目
    fn parse_output_line(&self, line: &str) -> Option<ParsedLogEntry>;

    /// 是否解析成功（检查退出码）
    fn check_success(&self, exit_code: i32) -> bool;

    /// 从日志列表中提取最终结果
    fn get_final_result(&self, logs: &[ParsedLogEntry]) -> Option<String>;

    /// 从日志列表中提取 usage 信息
    fn get_usage(&self, logs: &[ParsedLogEntry]) -> Option<ExecutionUsage>;
    fn get_model(&self) -> Option<String>;
}

/// 代码执行器注册表
pub struct ExecutorRegistry {
    executors: Arc<Mutex<HashMap<ExecutorType, Arc<dyn CodeExecutor>>>>,
}

impl ExecutorRegistry {
    pub fn new() -> Self {
        Self {
            executors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register<E: CodeExecutor + 'static>(&self, executor: E) {
        let executor_type = executor.executor_type();
        self.executors.lock().insert(executor_type, Arc::new(executor));
    }

    pub fn get(&self, executor_type: ExecutorType) -> Option<Arc<dyn CodeExecutor>> {
        self.executors.lock().get(&executor_type).cloned()
    }

    pub fn get_default(&self) -> Option<Arc<dyn CodeExecutor>> {
        self.get(ExecutorType::Claudecode)
    }

    pub fn list_executors(&self) -> Vec<ExecutorType> {
        self.executors.lock().keys().copied().collect()
    }
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 返回当前 UTC 时间的 ISO 8601 格式字符串 (2024-01-15T08:30:00.000Z)
pub fn get_timestamp() -> String {
    use chrono::Utc;
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// 去除 think 标签，只保留最终结论
pub fn extract_final_answer(content: &str) -> String {
    let re = regex::Regex::new(r"<think>[\s\S]*?</think>").unwrap();
    re.replace_all(content, "").trim().to_string()
}
