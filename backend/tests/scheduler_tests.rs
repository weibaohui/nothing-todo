//! Tests for scheduler logic - upsert/remove and job lifecycle

use std::sync::Arc;
use tokio::sync::broadcast;

use ntd::scheduler::TodoScheduler;

/// 测试无效的 cron 表达式会被 reject
#[tokio::test]
async fn test_upsert_invalid_cron_rejected() {
    let scheduler = TodoScheduler::new().await.unwrap();

    let result = scheduler
        .upsert_task(
            Arc::new(ntd::db::Database::new(":memory:").await.unwrap()),
            Arc::new(ntd::adapters::ExecutorRegistry::new()),
            broadcast::channel(16).0,
            1,
            "invalid cron".to_string(),
            Arc::new(ntd::task_manager::TaskManager::new()),
            Arc::new(tokio::sync::RwLock::new(ntd::config::Config::default())),
        )
        .await;

    assert!(result.is_err(), "invalid cron should be rejected");
    let err = result.unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("Invalid cron"),
        "error should mention invalid cron, got: {}",
        msg
    );
}

/// 测试有效的 cron 表达式可以被接受
#[tokio::test]
async fn test_upsert_valid_cron_accepted() {
    let scheduler = TodoScheduler::new().await.unwrap();

    let result = scheduler
        .upsert_task(
            Arc::new(ntd::db::Database::new(":memory:").await.unwrap()),
            Arc::new(ntd::adapters::ExecutorRegistry::new()),
            broadcast::channel(16).0,
            1,
            "0 */5 * * * *".to_string(), // every 5 minutes
            Arc::new(ntd::task_manager::TaskManager::new()),
            Arc::new(tokio::sync::RwLock::new(ntd::config::Config::default())),
        )
        .await;

    assert!(result.is_ok(), "valid cron should be accepted, got: {:?}", result.err());
}

/// 测试 remove_task_for_todo 对不存在的 todo 不会报错
#[tokio::test]
async fn test_remove_nonexistent_todo_is_noop() {
    let scheduler = TodoScheduler::new().await.unwrap();
    // 对于不存在的 todo_id，remove 不应 panic 或报错
    scheduler.remove_task_for_todo(999).await;
    // 如果没 panic，测试通过
}

/// 测试 upsert 会先 remove 旧的任务再添加新的（job_map 只保留最新的）
#[tokio::test]
async fn test_upsert_replaces_existing_task() {
    let scheduler = TodoScheduler::new().await.unwrap();

    // 先添加一个任务
    let result1 = scheduler
        .upsert_task(
            Arc::new(ntd::db::Database::new(":memory:").await.unwrap()),
            Arc::new(ntd::adapters::ExecutorRegistry::new()),
            broadcast::channel(16).0,
            1,
            "0 */5 * * * *".to_string(),
            Arc::new(ntd::task_manager::TaskManager::new()),
            Arc::new(tokio::sync::RwLock::new(ntd::config::Config::default())),
        )
        .await;
    assert!(result1.is_ok(), "first upsert should succeed");

    // 再次 upsert（同一个 todo_id，不同 cron）应当先 remove 旧的再添加新的
    let result2 = scheduler
        .upsert_task(
            Arc::new(ntd::db::Database::new(":memory:").await.unwrap()),
            Arc::new(ntd::adapters::ExecutorRegistry::new()),
            broadcast::channel(16).0,
            1,
            "0 0 * * * *".to_string(), // every hour
            Arc::new(ntd::task_manager::TaskManager::new()),
            Arc::new(tokio::sync::RwLock::new(ntd::config::Config::default())),
        )
        .await;
    assert!(result2.is_ok(), "second upsert should succeed");

    // 此时应该只有第二个任务存在
    scheduler.remove_task_for_todo(1).await;
    // 再次 remove 应当是 no-op（因为已被移除）
    scheduler.remove_task_for_todo(1).await;
}
