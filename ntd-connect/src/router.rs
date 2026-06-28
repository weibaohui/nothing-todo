//! Router trait：把 `IncomingMessage` 路由到正确的业务处理路径。
//!
//! # 与 backend MessageRouter 的关系
//!
//! backend 的 `services::message_router::MessageRouter` 是 router trait 的
//! 一个具体实现。Dispatcher 通过 `Arc<dyn Router>` 调用，编译期不知道
//! backend 类型。Dispatcher 的 worker 在 process_turn 之前先问 router
//! "这条消息归谁管"，根据 Decision 决定下一步：
//!
//! - [`Decision::Skip`] — 消息被丢弃（self / filter / 陈旧），不 spawn
//!   agent、不回 reply。
//! - [`Decision::Handled`] — router 自己处理完了（builtin 命令、slash
//!   命令等），dispatcher 不再做事。
//! - [`Decision::ForwardToAgent`] — 默认回复 / 项目绑定触发 todo 等，
//!   dispatcher 把消息发给 agent 执行。
//!
//! # 设计要点
//!
//! - trait 放在 ntd-connect 而非 backend：dispatcher 在 ntd-connect，
//!   不反向依赖 backend。
//! - `async_trait` + `Send + Sync`：router 可能持 DB / mpsc 等，
//!   要能在 tokio task 间共享。
//! - router 实现自己处理副作用（DB 读、debounce push 等），dispatcher
//!   只看 Decision 不关心内部。

use async_trait::async_trait;

use crate::types::IncomingMessage;

/// 路由结果：dispatcher 根据这个决定 worker 下一步动作。
///
/// 与 backend `services::message_router::Decision` 字段完全一致；
/// 这里定义在 ntd-connect 是为了让 dispatcher 不依赖 backend 类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// 跳过：消息不该被处理（self / disabled / 群白名单未命中）。
    Skip,
    /// 已处理：router 自己处理完了（builtin 命令 / slash 命令）。
    Handled,
    /// 转给 Agent 执行：默认回复 / 项目绑定触发 todo 等。
    ForwardToAgent,
}

/// Router trait：把消息路由到正确处理路径。
///
/// backend 实现这个 trait（参见 `backend/src/services/message_router.rs`）。
#[async_trait]
pub trait Router: Send + Sync {
    /// 路由一条入站消息。
    ///
    /// 调用方语义：
    /// - `Decision::Skip` → 立即 return，**不**调 agent、不回 reply
    /// - `Decision::Handled` → 立即 return（router 内部已回完 reply）
    /// - `Decision::ForwardToAgent` → 继续 process_turn（agent.send + events）
    async fn route(&self, msg: IncomingMessage) -> Decision;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decision 三种变体必须存在 + Debug + Clone + PartialEq + Eq。
    /// 编译期验证枚举派生声明 + 用法。
    #[test]
    fn test_decision_variants() {
        let skip = Decision::Skip;
        let handled = Decision::Handled;
        let fwd = Decision::ForwardToAgent;
        assert_ne!(skip, handled);
        assert_ne!(handled, fwd);
        assert_ne!(skip, fwd);
        // Clone + Debug 验证
        let _ = format!("{:?}", skip);
        let _ = skip.clone();
    }
}
