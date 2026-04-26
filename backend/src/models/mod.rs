use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TodoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TodoStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("unknown status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub prompt: String,
    pub status: TodoStatus,
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
    pub scheduler_next_run_at: Option<String>,
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

fn default_trigger_type() -> String { "manual".to_string() }

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
    #[serde(default = "default_trigger_type")]
    pub trigger_type: String,
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
    pub total_cache_read_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cost_usd: Option<f64>,
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
    pub prompt: String,
    #[serde(default)]
    pub tag_ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct UpdateTodoRequest {
    pub title: String,
    pub prompt: String,
    pub status: TodoStatus,
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
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecordsPage {
    pub records: Vec<ExecutionRecord>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
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
    Codebuddy,
    Opencode,
}

impl Default for ExecutorType {
    fn default() -> Self {
        ExecutorType::Claudecode
    }
}

impl ExecutorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutorType::Joinai => "joinai",
            ExecutorType::Claudecode => "claudecode",
            ExecutorType::Codebuddy => "codebuddy",
            ExecutorType::Opencode => "opencode",
        }
    }
}

impl std::fmt::Display for ExecutorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Unified API Response
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub data: Option<T>,
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { code: 0, data: Some(data), message: "ok".to_string() }
    }

    pub fn err(code: i32, message: &str) -> Self {
        Self { code, data: None, message: message.to_string() }
    }
}

// Business error codes
pub mod codes {
    pub const NOT_FOUND: i32 = 40001;
    pub const BAD_REQUEST: i32 = 40002;
    pub const INTERNAL: i32 = 50001;
}
