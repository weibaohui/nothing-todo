//! Zhanlu-specific event parsing.
//!
//! Zhanlu 复用了 OpenCode 的事件格式（hyphenated event types，例如 step-start、tool-use），
//! 并额外使用 camelCase 字段名（如 sessionID）。
//!
//! Issue #673 要求「行为跟 opencode 完全一致，执行命令也一致，返回输出 JSON 格式也一致」，
//! 所以这里只把类型名从 Opencode 前缀重命名为 Zhanlu，serde 结构和字段映射保持完全一致。

use std::collections::HashMap;
use serde::Deserialize;

/// Zhanlu agent event with hyphenated type names (与 OpenCode 完全相同的 JSON 结构)
#[derive(Debug, Clone, Deserialize)]
pub struct ZhanluAgentEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub timestamp: Option<u64>,
    #[serde(default, rename = "sessionID")]
    pub session_id: Option<String>,
    #[serde(default)]
    pub part: Option<ZhanluAgentPart>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZhanluAgentPart {
    #[serde(rename = "type")]
    pub part_type: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub call_id: Option<String>,
    #[serde(default)]
    pub state: Option<ZhanluAgentToolState>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub tokens: Option<ZhanluAgentTokens>,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZhanluAgentToolState {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub input: Option<ZhanluAgentToolInput>,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZhanluAgentToolInput {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ZhanluAgentToolInput {
    pub fn to_full_json(&self) -> String {
        let mut map = serde_json::Map::new();
        if let Some(ref cmd) = self.command {
            map.insert("command".into(), serde_json::Value::String(cmd.clone()));
        }
        if let Some(ref desc) = self.description {
            map.insert("description".into(), serde_json::Value::String(desc.clone()));
        }
        for (k, v) in &self.extra {
            map.insert(k.clone(), v.clone());
        }
        serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZhanluAgentTokens {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    #[serde(default)]
    pub reasoning: u64,
    #[serde(default)]
    pub cache: ZhanluAgentCacheTokens,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ZhanluAgentCacheTokens {
    #[serde(default)]
    pub read: u64,
    #[serde(default)]
    pub write: u64,
}