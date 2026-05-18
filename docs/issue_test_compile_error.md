# Issue: 集成测试 api_integration_test.rs 编译错误

## 问题描述
`backend/tests/api_integration_test.rs` 调用 `load_from_db` 方法时只提供了 4 个参数，但该方法实际需要 5 个参数（包括 `app_config`）。

## 错误信息
```
error[E0061]: this method takes 5 arguments but 4 arguments were supplied
  --> tests/api_integration_test.rs:31:10
   |
31 |         .load_from_db(db.clone(), executor_registry.clone(), tx.clone(), task_manager.clone())
   |          ^^^^^^^^^^^^------------------------------------------------------------------------- argument #5 of type `Arc<tokio::sync::RwLock<ntd::config::Config>>` is missing
```

## 根因分析
`scheduler.rs` 中的 `load_from_db` 方法签名：
```rust
pub async fn load_from_db(
    &self,
    db: Arc<Database>,
    executor_registry: Arc<ExecutorRegistry>,
    tx: broadcast::Sender<ExecEvent>,
    task_manager: Arc<TaskManager>,
    app_config: Arc<tokio::sync::RwLock<Config>>,  // 第5个参数
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

测试代码只传了 4 个参数，缺少 `app_config`。

## 影响
- 集成测试无法编译通过
- `cargo test` 命令失败

## 最小修复方案
在 `api_integration_test.rs:31` 添加缺失的 `app_config` 参数。

参考第 36 行已有 `let config = Arc::new(tokio::sync::RwLock::new(Config::default()));`，可直接传入。

## 修复状态
- [ ] 待修复
- [ ] PR 已创建
- [ ] 已合并
