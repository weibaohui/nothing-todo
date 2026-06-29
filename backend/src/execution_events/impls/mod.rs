//! 各执行器的事件提取器实现
//!
//! 每个执行器对应一个独立的模块，实现 EventExtractor trait。

pub mod claude_code;
pub mod codex;
pub mod default;
pub mod hermes;
pub mod kilo;
pub mod kimi;
pub mod opencode;

pub use claude_code::ClaudeCodeExtractor;
pub use codex::CodexExtractor;
pub use default::DefaultExtractor;
pub use hermes::HermesExtractor;
pub use kilo::KiloExtractor;
pub use kimi::KimiExtractor;
pub use opencode::OpencodeExtractor;
