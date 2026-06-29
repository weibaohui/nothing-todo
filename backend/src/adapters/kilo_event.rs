//! Kilo-specific event parsing.
//!
//! Kilo 复用了 OpenCode 的事件格式（hyphenated event types，例如 step-start、tool-use），
//! 并额外使用 camelCase 字段名（如 sessionID）。
//!
//! 行为与 Opencode/Zhanlu 完全一致：相同的命令参数、相同的 JSON 输出格式、
//! 相同的退出码语义（非零但含 step_finish 事件时视为成功）。
//! 所以这里只把类型名从 Opencode 前缀重命名为 Kilo，serde 结构和字段映射保持完全一致。

use std::collections::HashMap;
use serde::Deserialize;

/// Kilo agent event with hyphenated type names (与 OpenCode 完全相同的 JSON 结构)
#[derive(Debug, Clone, Deserialize)]
pub struct KiloAgentEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub timestamp: Option<u64>,
    #[serde(default, rename = "sessionID")]
    pub session_id: Option<String>,
    #[serde(default)]
    pub part: Option<KiloAgentPart>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KiloAgentPart {
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
    pub state: Option<KiloAgentToolState>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub tokens: Option<KiloAgentTokens>,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KiloAgentToolState {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub input: Option<KiloAgentToolInput>,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KiloAgentToolInput {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl KiloAgentToolInput {
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
pub struct KiloAgentTokens {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    #[serde(default)]
    pub reasoning: u64,
    #[serde(default)]
    pub cache: KiloAgentCacheTokens,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct KiloAgentCacheTokens {
    #[serde(default)]
    pub read: u64,
    #[serde(default)]
    pub write: u64,
}
