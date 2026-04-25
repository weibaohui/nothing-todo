use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tag_ids: Vec<i64>,
    #[serde(default)]
    pub executor: Option<String>,
    #[serde(default)]
    pub scheduler_enabled: bool,
    #[serde(default)]
    pub scheduler_config: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: i64,
    pub todo_id: i64,
    pub status: String,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub logs: String,
    pub result: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub usage: Option<ExecutionUsage>,
    pub executor: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub todo_id: i64,
    pub total_executions: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub running_count: i64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    #[serde(rename = "type")]
    pub log_type: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedLogEntry {
    pub timestamp: String,
    #[serde(rename = "type")]
    pub log_type: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ExecutionUsage>,
}

// Request/Response types
#[derive(Deserialize)]
pub struct CreateTodoRequest {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub tag_ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct UpdateTodoRequest {
    pub title: String,
    pub description: String,
    pub status: String,
    #[serde(default)]
    pub executor: Option<String>,
    #[serde(default)]
    pub scheduler_enabled: Option<bool>,
    #[serde(default)]
    pub scheduler_config: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTagsRequest {
    pub tag_ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: String,
}

#[derive(Deserialize)]
pub struct ExecuteRequest {
    pub todo_id: i64,
    pub message: String,
    pub executor: Option<String>,
}

#[derive(Deserialize)]
pub struct TodoIdQuery {
    pub todo_id: i64,
}

#[derive(Deserialize)]
pub struct UpdateSchedulerRequest {
    pub scheduler_enabled: bool,
    pub scheduler_config: Option<String>,
}

// Executor types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ExecutorType {
    Joinai,
    Claudecode,
}

impl Default for ExecutorType {
    fn default() -> Self {
        ExecutorType::Claudecode
    }
}

impl std::fmt::Display for ExecutorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutorType::Joinai => write!(f, "joinai"),
            ExecutorType::Claudecode => write!(f, "claudecode"),
        }
    }
}
