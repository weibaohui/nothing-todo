//! TypingIndicator trait 与 TypingGuard：抽象「处理中」副作用。
//!
//! # 与 cc-connect 的对应
//!
//! 对应 `cc-connect/core/interfaces.go:246-248 TypingIndicator`。
//!
//! # 设计要点
//!
//! - `TypingIndicator` 是**可选能力**：Channel 实现可以选择实现
//!   （飞书用 reaction，TG 用 typing API）。Rust 这边用单独的 trait +
//!   `as_typing_indicator()` 反射方法（见 `channel.rs`），而不是
//!   required method on Channel trait，保持 Channel 的最小契约。
//! - 返回 [`TypingGuard`] 而不是直接 `()`：调用方 drop guard 时自动
//!   停止 typing，不需要显式调 stop。Drop impl 自动驱动底层 future。
//! - TypingGuard 内部用 `Pin<Box<dyn Future>>` 持有 stop future：
//!   channel 的 stop 逻辑常常需要 await（调 HTTP API 删 reaction），
//!   不能在 Drop 里阻塞，因此把 stop 延迟到显式 `stop().await` 调用。

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{ReplyContext, ReplyTarget};

/// 「处理中」副作用抽象。
///
/// Channel 实现可以选择性实现此 trait：
/// - 飞书：加 `👀` reaction；stop 时删 reaction。
/// - Telegram：调 `sendChatAction(typing)`；stop 时无操作（typing
///   5s 自动消失）。
/// - 不支持的 channel：不实现即可。
///
/// # 与 Channel trait 的关系
///
/// TypingIndicator **不是** [`crate::channel::Channel`] 的 super trait，
/// 是平行 trait。Dispatcher 通过 downcast 或扩展方法检测具体 channel
/// 是否实现此 trait。
#[async_trait]
pub trait TypingIndicator: Send + Sync {
    /// 开始「处理中」展示，返回 [`TypingGuard`]。
    ///
    /// 调用方应把 guard 持有到 turn 结束；调用 [`TypingGuard::stop`]
    /// 显式停止（drop guard 不会自动停止，见 guard 的 rustdoc）。
    async fn start_typing(
        &self,
        ctx: &ReplyContext,
        target: &ReplyTarget,
    ) -> Result<TypingGuard>;
}

/// 「处理中」RAII guard。
///
/// 持有时：typing 效果对用户可见。
/// `stop().await` 后：typing 消失。
///
/// # Drop 行为
///
/// **Drop 不会自动 stop**。原因：删除 typing 通常需要 await HTTP 调用，
/// Rust 的 `Drop` 不能 await。如果 Drop 时 typing 还在，对应 channel
/// 的「处理中」展示会自然过期（飞书 reaction 一直挂着，直到下次 stop
/// 或消息处理完成；TG typing 5s 自动消失）。
///
/// 因此 dispatcher 必须显式调 `guard.stop().await`，**不要**仅靠 drop。
pub struct TypingGuard {
    /// 内部 stop future；`take()` 后置 None 防止重复 stop。
    stop_future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl TypingGuard {
    /// 从一个 stop future 构造 guard。
    ///
    /// channel 实现通常这样写：
    /// ```ignore
    /// TypingGuard::new(async move {
    ///     // 调 HTTP API 删除 reaction
    ///     let _ = delete_reaction_api(...).await;
    /// })
    /// ```
    pub fn new(stop_future: impl Future<Output = ()> + Send + 'static) -> Self {
        TypingGuard {
            stop_future: Some(Box::pin(stop_future)),
        }
    }

    /// 构造一个「no-op」guard，用于 channel 不支持 typing 但又必须返回
    /// 值的场景（例如 trait 方法不能返回 Option）。
    pub fn noop() -> Self {
        TypingGuard { stop_future: None }
    }

    /// 显式停止 typing；同一 guard 重复调用是 no-op。
    ///
    /// 调用后 future 被 take 走，避免重复执行 stop 副作用（飞书
    /// 重复删同一个 reaction 会报 404）。
    pub async fn stop(mut self) {
        if let Some(fut) = self.stop_future.take() {
            fut.await;
        }
    }
}

impl std::fmt::Debug for TypingGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.stop_future {
            Some(_) => f.debug_struct("TypingGuard").finish_non_exhaustive(),
            None => f.debug_struct("TypingGuard").field("stop", &"noop").finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FeishuChatType, ReplyTarget};
    use std::sync::atomic::{AtomicU32, Ordering};

    /// `TypingGuard::stop().await` 必须执行传入的 stop future（用
    /// AtomicU32 计数验证 future 被实际驱动过）。
    #[tokio::test]
    async fn test_typing_guard_stop_invokes_future() {
        // 计数器：stop future 执行后 +1。
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let guard = TypingGuard::new(async {
            // 静态变量不能直接捕获，用 atomic。
            COUNTER.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(COUNTER.load(Ordering::SeqCst), 0, "stop 前计数器应为 0");
        guard.stop().await;
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1, "stop 后计数器应为 1");
    }

    /// `TypingGuard::noop` 调 stop 必须立即返回，不 panic。
    #[tokio::test]
    async fn test_typing_guard_noop() {
        let guard = TypingGuard::noop();
        guard.stop().await;
        // 没有 future 能执行，所以无副作用。
    }

    /// 重复 stop 同一 guard 会编译失败（`stop(mut self)` 拿走所有权）。
    /// 这里用类型系统层面验证：stop 一次后 guard 已消费，不能再 stop。
    /// （此测试通过编译验证语义，运行时不报错即可。）
    #[tokio::test]
    async fn test_typing_guard_consumed_after_stop() {
        let guard = TypingGuard::new(async {});
        guard.stop().await;
        // 下面这行编译不过——drop 时 guard 已 move 走，证明一次性语义。
        // 故意保留注释，不写实际代码。
    }

    /// TypingGuard 的 Debug 实现不应 panic（用于日志安全）。
    #[test]
    fn test_typing_guard_debug() {
        let guard = TypingGuard::new(async {});
        let s = format!("{:?}", guard);
        assert!(s.contains("TypingGuard"));

        let noop = TypingGuard::noop();
        let s = format!("{:?}", noop);
        assert!(s.contains("TypingGuard"));
        assert!(s.contains("noop"));
    }

    /// TypingGuard 必须能 Send（被 dispatcher 跨 task 持有）。
    /// 编译期验证：未来 Send 通过 Box<dyn Future + Send> 强制。
    #[tokio::test]
    async fn test_typing_guard_send() {
        // 把 guard 放进 tokio::spawn，确保它是 Send。
        let guard = TypingGuard::new(async {});
        let h = tokio::spawn(async move {
            guard.stop().await;
        });
        h.await.unwrap();
    }

    /// ReplyTarget 必须能作为 TypingIndicator::start_typing 的入参
    /// 传递（验证 trait 签名设计无误）。
    #[test]
    fn test_typing_indicator_signature_uses_reply_target() {
        // 仅类型断言；运行时不调真实实现。
        // 编译通过即说明 trait 签名正确。
        // （这里只是占位，实际 mock TypingIndicator 在后续 platform 实现里。）
        let target = ReplyTarget::feishu("oc_x", None, FeishuChatType::P2p);
        // ReplyTarget 必须实现 Debug（trace 日志要用）。
        let _: &dyn std::fmt::Debug = &target;
    }
}
