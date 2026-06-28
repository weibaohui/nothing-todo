//! 共享 HTTP client。
//!
//! # 设计动机
//!
//! 当前 nothing-todo 的飞书实现里多次出现
//! `let client = reqwest::Client::new();`（feishu_listener.rs 6 处），
//! 每次新建 client 都会断开连接池、强制重新 TCP+TLS 握手，是飞书
//! reaction 串行阻塞的次要根因之一。
//!
//! 本模块提供 [`SharedHttpClient`]，要求 Channel / Agent / Dispatcher
//! 都从它拿底层 `reqwest::Client`，让连接池在 crate 内统一复用。
//!
//! # 复用策略
//!
//! `reqwest::Client::clone()` 本身是廉价的（内部 Arc），所以
//! [`SharedHttpClient`] 直接 `#[derive(Clone)]` 就够了，不需要外面再
//! 包一层 `Arc<SharedHttpClient>`。多个 channel 实例共享同一个 client
//! 时，调用 `SharedHttpClient::clone()` 即可。

use std::time::Duration;

/// 进程级共享的 HTTP client。
///
/// 内部包一个 `reqwest::Client`；连接池参数（连接数、超时、TLS 后端）
/// 在 [`SharedHttpClient::new`] 里统一设置，避免各处魔法数字。
#[derive(Clone)]
pub struct SharedHttpClient {
    inner: reqwest::Client,
}

impl SharedHttpClient {
    /// 构造一个默认配置的 client。
    ///
    /// 参数选择理由：
    /// - `pool_max_idle_per_host(8)`：飞书 API 单 host 连接够用，
    ///   设太大反而浪费 fd。
    /// - `timeout(30s)`：覆盖整个请求生命周期（connect + send + recv）；
    ///   飞书 reaction 接口偶发慢响应（>10s）实测存在，30s 是宽松上限。
    /// - TLS 后端由 `Cargo.toml` 的 `rustls-tls` feature 决定，避开系统
    ///   openssl 依赖（与 backend 对齐）。
    pub fn new() -> Self {
        let inner = reqwest::Client::builder()
            .pool_max_idle_per_host(8)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest::Client with default config should build");
        SharedHttpClient { inner }
    }

    /// 暴露底层 `reqwest::Client` 引用；调用方可以拿去做任意 HTTP。
    ///
    /// 不返回所有权是有意为之：避免下游不小心把 client move 走再 `new()`
    /// 一个新的。共享语义靠引用传递表达。
    pub fn raw(&self) -> &reqwest::Client {
        &self.inner
    }
}

impl Default for SharedHttpClient {
    fn default() -> Self {
        SharedHttpClient::new()
    }
}

impl std::fmt::Debug for SharedHttpClient {
    // reqwest::Client 没有 Debug impl，只能手动给一个无信息的 stub。
    // 避免 Debug derive 报错，同时不泄露内部 token / cookie（即便没有）。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedHttpClient").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new()` 必须能成功构造，不能 panic。
    #[test]
    fn test_new_succeeds() {
        let c = SharedHttpClient::new();
        // raw() 返回的引用不应为空。
        let _ = c.raw();
    }

    /// Clone 必须廉价（内部 Arc 共享，不应触发 `new()`）。
    /// 间接验证：克隆后再 clone，两份都应能用。
    #[test]
    fn test_clone_shares_underlying() {
        let a = SharedHttpClient::new();
        let b = a.clone();
        // 两者 raw() 指针应指向同一份 reqwest::Client（reqwest 内部 Arc）。
        // 由于 reqwest::Client 没暴露 Arc::strong_count，这里只能验证
        // 引用都能用、Clone 不会触发 panic。
        let _ = a.raw();
        let _ = b.raw();
    }

    /// Default 应等价于 new()。允许调用方写 `SharedHttpClient::default()`
    /// 而不是显式 `new()`（例如在结构体字段默认值场景）。
    #[test]
    fn test_default_matches_new() {
        let a = SharedHttpClient::default();
        let b = SharedHttpClient::new();
        // 没有直接的方法比较两个 Client 内容；只能确保两者都能用。
        let _ = a.raw();
        let _ = b.raw();
    }

    /// Debug 输出不应为空字符串，避免在日志里看到空 placeholder。
    #[test]
    fn test_debug_non_empty() {
        let s = format!("{:?}", SharedHttpClient::new());
        assert!(!s.is_empty());
        assert!(s.contains("SharedHttpClient"));
    }
}
