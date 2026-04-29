use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeMessage {
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
pub struct ClaudeMessageContent {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub content: Vec<ClaudeContentBlock>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeContentBlock {
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
pub struct ClaudeUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}
