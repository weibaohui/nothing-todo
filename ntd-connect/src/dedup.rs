//! 按 message_id 去重（LRU + TTL）。
//!
//! # 为什么需要
//!
//! IM 平台（尤其飞书）在网络抖动时可能重投同一条消息：WS 帧断线重连
//! 后飞书会重发最近 N 条事件，cc-connect 通过 dedup 兜住这种情况
//!（`core/dedup.go:8-44`）。本模块提供等价实现。
//!
//! # 用法
//!
//! - `check_and_set(key)` 在 dispatcher 处理每条消息前调用一次：
//!   返回 `true` 表示首次见到、`false` 表示重复。
//! - 周期性调用 `cleanup_expired()` 清理过期 key（或者由 dispatcher
//!   启动后台 task 自动清理，见 [`Dedup::spawn_cleanup_task`]）。

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use lru::LruCache;
use parking_lot::Mutex;

/// 默认容量。4096 是 cc-connect 经验值；足够 30 分钟内飞书重投窗口。
const DEFAULT_CAPACITY: usize = 4096;

/// Dedup 状态：LRU cache + 上次清理时间。
struct DedupState {
    /// key = message_id, value = 首次写入时刻。
    cache: LruCache<String, Instant>,
}

/// 进程级 dedup。
///
/// 内部用 `parking_lot::Mutex` 保护 `LruCache`：lock 持有时间仅一两次
/// `LruCache::get/put`，不开 await，因此 sync mutex 是合适的选择。
/// 调用方不需要 `&mut self`，允许在 `Arc<Dedup>` 多处共享。
pub struct Dedup {
    state: Mutex<DedupState>,
    /// key 过期时间（last_seen + ttl < now 即视为过期）。
    ttl: Duration,
}

impl Dedup {
    /// 用默认容量（4096）构造。
    pub fn new(ttl: Duration) -> Self {
        Self::with_capacity(ttl, DEFAULT_CAPACITY)
    }

    /// 指定 LRU 容量构造。
    ///
    /// `capacity` 是 cache 持有的最大 key 数；超出时按 LRU 驱逐最近
    /// 最少使用的 key。注意 LRU 驱逐会无视 TTL，所以 capacity 应该远
    /// 大于「TTL 内可能收到的最大 distinct message 数」。
    pub fn with_capacity(ttl: Duration, capacity: usize) -> Self {
        // LruCache 要求 NonZeroUsize；用 NonZero::new 把 usize 转过来。
        // 容量 0 没有意义（永远 evict 刚 put 的），这里 expect 是合理的契约保证。
        let nz = NonZeroUsize::new(capacity)
            .expect("Dedup capacity must be > 0; pass DEFAULT_CAPACITY or larger");
        Dedup {
            state: Mutex::new(DedupState {
                cache: LruCache::new(nz),
            }),
            ttl,
        }
    }

    /// 检查 key 是否已存在；不存在则写入当前时间，返回 `true`。
    ///
    /// 这是 dedup 的**唯一**入口；调用方语义上视为：
    /// ```text
    /// if !dedup.check_and_set(&msg.raw_message_id) { return; /* 重复 */ }
    /// ```
    pub fn check_and_set(&self, key: &str) -> bool {
        let mut state = self.state.lock();
        if state.cache.contains(key) {
            // 已存在：判为重复，但不刷新时间戳。
            // 设计取舍：cc-connect 在重复命中时是否刷新 last_seen
            // 没明确写。从「相同 key 应有相同的过期时刻」语义看，
            // 不刷新更可预测。
            return false;
        }
        state.cache.put(key.to_string(), Instant::now());
        true
    }

    /// 检查 key 是否已存在（不写入）；用于「先 peek 再决定是否处理」场景。
    pub fn contains(&self, key: &str) -> bool {
        self.state.lock().cache.contains(key)
    }

    /// 清理已过期的 key，返回被清理的条目数。
    ///
    /// 复杂度 O(n)，调用方应周期性（例如每 30s）调用一次；不要在
    /// 每条消息路径上调用，避免性能抖动。
    pub fn cleanup_expired(&self) -> usize {
        let mut state = self.state.lock();
        let now = Instant::now();
        // 先收集要删的 key，再统一 pop，避免持锁时迭代器失效。
        let expired: Vec<String> = state
            .cache
            .iter()
            .filter(|(_, ts)| now.duration_since(**ts) >= self.ttl)
            .map(|(k, _)| k.clone())
            .collect();
        let n = expired.len();
        for k in &expired {
            state.cache.pop(k);
        }
        n
    }

    /// 启动后台清理 task：每隔 `interval` 跑一次 [`Dedup::cleanup_expired`]。
    ///
    /// 返回 [`tokio::task::JoinHandle`]，调用方负责在 shutdown 时 `abort()`
    /// 或持有 handle 让进程退出时自然清理。
    ///
    /// 设计上仅在 dispatcher 启动时调用一次；不暴露全局注册入口，
    /// 避免多个调用方重复启动多个清理 task。
    pub fn spawn_cleanup_task(
        self: Arc<Self>,
        interval: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // 第一次 tick 立即触发；之后按 interval 周期。
            // Skip MissedTickBehavior::Skip 是默认行为，避免清理堆积。
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let removed = self.cleanup_expired();
                if removed > 0 {
                    tracing::debug!(
                        "dedup cleanup removed {removed} expired entries (ttl={:?})",
                        self.ttl
                    );
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 同 key 第二次返回 false（重复检测基本语义）。
    #[test]
    fn test_check_and_set_dedup() {
        let d = Dedup::with_capacity(Duration::from_secs(60), 16);
        assert!(d.check_and_set("m1"));
        assert!(!d.check_and_set("m1"));
        // 不同 key 各自独立。
        assert!(d.check_and_set("m2"));
        assert!(!d.check_and_set("m2"));
    }

    /// `contains` 是只读探针，不写入。
    #[test]
    fn test_contains_does_not_write() {
        let d = Dedup::with_capacity(Duration::from_secs(60), 16);
        assert!(!d.contains("m1"));
        // contains 后再 check_and_set 仍然返回 true（未被 contains 写入）。
        assert!(d.check_and_set("m1"));
        // 第一次 contains 后再 contains 仍为 true（已写入）。
        assert!(d.contains("m1"));
    }

    /// LRU 容量上限：超出时旧的 key 被驱逐，重新出现时视为新 key。
    #[test]
    fn test_lru_eviction() {
        let d = Dedup::with_capacity(Duration::from_secs(60), 2);
        assert!(d.check_and_set("a"));
        assert!(d.check_and_set("b"));
        // 容量满了，再插入一个会驱逐最旧的 "a"。
        assert!(d.check_and_set("c"));
        // "a" 被驱逐，重新出现时应判为新 key。
        assert!(d.check_and_set("a"));
        // "b" 仍在 LRU 中（被驱逐顺序取决于访问模式）。
        // 这里不严格断言 b/c 的状态，避免测试与 LRU 实现细节耦合。
    }

    /// cleanup_expired 删除 ttl 之前的 key，保留 ttl 内的。
    /// 用极短 TTL（10ms）+ sleep 让测试快。
    #[tokio::test]
    async fn test_cleanup_expired_removes_old() {
        let d = Dedup::with_capacity(Duration::from_millis(10), 16);
        assert!(d.check_and_set("expired"));
        // sleep 超过 ttl。
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(d.check_and_set("fresh"));

        let removed = d.cleanup_expired();
        assert_eq!(removed, 1, "应清理掉 1 条已过期 key");
        // "expired" 被清后，重新出现应判为新 key。
        assert!(d.check_and_set("expired"));
    }

    /// 后台 cleanup task 必须能跑起来（不会 panic / 不会卡死）。
    /// 用 50ms 间隔跑 200ms 后 abort。
    #[tokio::test]
    async fn test_spawn_cleanup_task_runs() {
        let d = Arc::new(Dedup::with_capacity(Duration::from_millis(10), 16));
        d.check_and_set("x");
        let handle = d.clone().spawn_cleanup_task(Duration::from_millis(50));
        // 等几个 tick。
        tokio::time::sleep(Duration::from_millis(180)).await;
        handle.abort();
        // task 已 abort；不验证具体清理数量（时序敏感）。
        // 只要 spawn 不 panic、循环能跑几个 tick 就算通过。
        let _ = handle.await;
    }
}
