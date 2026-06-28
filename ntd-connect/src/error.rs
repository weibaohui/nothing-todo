//! ntd-connect 统一错误类型。
//!
//! 用 `thiserror` 派生 `std::error::Error` + `Display`；crate 内一律用
//! [`Result`],避免上层到处写 `Result<T, ntd_connect::error::Error>`。
//!
//! 设计原则：
//! - 每个 variant 对应一类可恢复失败；不要把所有错塞进 `Other`。
//! - `From` impl 仅覆盖「crate 边界」的常见转换（IO / JSON），避免
//!   第三方库的具体错误穿透上来污染 API 表面。

use std::io;
use serde_json;

/// ntd-connect 统一错误类型。
///
/// 备注：
/// - `Platform(String)` / `Agent(String)` 用字符串承载 platform-/agent-specific
///   错误，避免强耦合具体实现（飞书/钉钉的错误码未来可能再分裂）。
/// - `ChannelClosed` 表示 channel 长连接断开；上层可选择重连。
/// - `QueueFull` 表示 per-session busy queue 容量已满，调用方应丢弃消息。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO 错误（std::io::Error 直接透传）。
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// JSON 序列化/反序列化失败。
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Channel 平台层错误（飞书/钉钉/TG 等的具体失败）。
    #[error("platform error: {0}")]
    Platform(String),

    /// Agent executor 层错误（Claude Code / Codex / Hermes 等）。
    #[error("agent error: {0}")]
    Agent(String),

    /// Channel 长连接已关闭；上层需要重连或停机。
    #[error("channel closed")]
    ChannelClosed,

    /// Per-session busy queue 容量已满；消息应丢弃。
    #[error("session queue full")]
    QueueFull,

    /// Session 仍持有锁，新消息已被 watermark 判为陈旧。
    #[error("stale message rejected by watermark")]
    StaleMessage,

    /// Agent 请求的权限被拒绝或超时未回执。
    #[error("permission denied for request {0}")]
    PermissionDenied(String),

    /// 兜底：未分类的错误。**不**鼓励新增调用点；先想能否映射到已有 variant。
    #[error("other: {0}")]
    Other(String),
}

impl Error {
    /// 把任意字符串包成 `Error::Other`；用于「快速失败」分支。
    pub fn other(msg: impl Into<String>) -> Self {
        Error::Other(msg.into())
    }

    /// 构造平台层错误。
    pub fn platform(msg: impl Into<String>) -> Self {
        Error::Platform(msg.into())
    }

    /// 构造 agent 层错误。
    pub fn agent(msg: impl Into<String>) -> Self {
        Error::Agent(msg.into())
    }
}

/// ntd-connect 统一 `Result` 类型别名。
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    /// `Error::Io` 必须能从 `std::io::Error` 自动转换，方便 IO 边界
    /// 直接 `?` 透传，不写手工 map_err。
    #[test]
    fn test_from_io_error() {
        // 用一个不存在的路径触发 NotFound，作为 IO 错误的最小复现。
        let io_err = io::Error::new(io::ErrorKind::NotFound, "missing");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        // Display 输出必须包含原始消息，方便日志直接定位。
        assert!(err.to_string().contains("missing"));
    }

    /// `Error::Json` 从 serde_json 转换；保证序列化失败能直接 `?`。
    #[test]
    fn test_from_serde_error() {
        let json_err = serde_json::from_str::<i32>("not-a-number").unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Json(_)));
    }

    /// `Error::Platform` / `Agent` / `Other` 的便捷构造器验证。
    #[test]
    fn test_convenience_constructors() {
        assert!(matches!(Error::platform("x"), Error::Platform(s) if s == "x"));
        assert!(matches!(Error::agent("y"), Error::Agent(s) if s == "y"));
        assert!(matches!(Error::other("z"), Error::Other(s) if s == "z"));
    }

    /// `QueueFull` / `ChannelClosed` / `StaleMessage` 等无载荷 variant 的
    /// Display 必须是非空字符串（`thiserror` 默认派生即可）。
    #[test]
    fn test_unit_variants_display() {
        assert!(!Error::ChannelClosed.to_string().is_empty());
        assert!(!Error::QueueFull.to_string().is_empty());
        assert!(!Error::StaleMessage.to_string().is_empty());
    }
}
