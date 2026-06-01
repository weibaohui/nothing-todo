use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Semaphore, OwnedSemaphorePermit};
use tracing::{error, info, warn};

use crate::db::Database;
use crate::executor_service::RunTodoExecutionRequest;
use crate::handlers::execution::start_todo_execution;
use crate::hooks::db::HookDb;
use crate::hooks::models::*;
use crate::service_context::ServiceContext;

pub struct HookService {
    ctx: ServiceContext,
    semaphore: Arc<Semaphore>,
}

impl HookService {
    pub fn new(ctx: ServiceContext, max_concurrency: u64, _default_timeout_secs: u64) -> Self {
        Self {
            ctx,
            semaphore: Arc::new(Semaphore::new(max_concurrency as usize)),
        }
    }

    /// Fire before_* hooks synchronously - returns error if any hook fails
    pub async fn fire_before_hooks(
        &self,
        ctx: &HookContext,
        tag_ids: &[i64],
    ) -> Result<(), String> {
        let rules = self.get_matching_rules(ctx.trigger, ctx, tag_ids).await?;

        // Only execute sync (before_*) hooks
        let sync_hooks: Vec<_> = rules.into_iter().filter(|r| r.trigger.is_sync()).collect();

        for rule in sync_hooks {
            if !self.matches_filter(&rule.filter, ctx, tag_ids) {
                continue;
            }

            let permit = self.acquire_permit().await;

            let result = self
                .execute_hook(&rule, ctx)
                .await;

            drop(permit);

            if result.success {
                info!(
                    hook_id = rule.id,
                    hook_name = %rule.name,
                    todo_id = ctx.todo_id,
                    "before hook executed successfully"
                );
            } else {
                let msg = format!(
                    "Hook '{}' failed: {}",
                    rule.name,
                    result.error_msg.unwrap_or_else(|| "unknown error".to_string())
                );
                error!(hook_id = rule.id, "{}", msg);
                return Err(msg);
            }
        }

        Ok(())
    }

    /// Fire after_* hooks asynchronously - does not block
    pub fn fire_after_hooks(self: Arc<Self>, ctx: HookContext, tag_ids: Vec<i64>) {
        let this = self.clone();

        tokio::spawn(async move {
            let trigger = ctx.trigger;
            if !trigger.is_sync() {
                // Use get_matching_rules to respect per-todo hook config (inherit/custom/disabled)
                let rules = match this.get_matching_rules(trigger, &ctx, &tag_ids).await {
                    Ok(r) => r.into_iter().filter(|r| !r.trigger.is_sync()).collect::<Vec<_>>(),
                    Err(_) => return,
                };

                for rule in rules {
                    if !this.matches_filter(&rule.filter, &ctx, &tag_ids) {
                        continue;
                    }

                    let permit = this.acquire_permit().await;
                    let ctx_clone = ctx.clone();
                    let db_clone = this.ctx.db.clone();
                    let executor_registry = this.ctx.executor_registry.clone();
                    let tx = this.ctx.tx.clone();
                    let task_manager = this.ctx.task_manager.clone();
                    let config = this.ctx.config.clone();

                    tokio::spawn(async move {
                        let start = Instant::now();
                        let result = execute_target_todo(
                            &db_clone,
                            &executor_registry,
                            tx,
                            &task_manager,
                            &config,
                            &rule,
                            &ctx_clone,
                        )
                        .await;
                        let duration_ms = start.elapsed().as_millis() as i64;

                        let args_json = serde_json::to_string(&ctx_clone).ok();
                        let _ = HookDb::insert_hook_log(
                            &db_clone.conn,
                            rule.id,
                            Some(rule.name.clone()),
                            ctx_clone.trigger.as_str(),
                            ctx_clone.todo_id,
                            args_json.as_deref(),
                            None,
                            result.exit_code,
                            Some(&result.stdout),
                            Some(&result.stderr),
                            duration_ms,
                            result.success,
                            result.error_msg.as_deref(),
                        )
                        .await;

                        drop(permit);
                    });
                }
            }
        });
    }

    async fn get_matching_rules(
        &self,
        trigger: HookTrigger,
        ctx: &HookContext,
        _tag_ids: &[i64],
    ) -> Result<Vec<HookRule>, String> {
        // Get global config
        let global_config = HookDb::get_global_config(&self.ctx.db.conn)
            .await
            .map_err(|e| e.to_string())?;

        if !global_config.enabled {
            return Ok(vec![]);
        }

        // Get hooks by trigger
        let mut rules = HookDb::get_hooks_by_trigger(&self.ctx.db.conn, trigger)
            .await
            .map_err(|e| e.to_string())?;

        // If this todo has custom hooks, use those instead
        if let Some(todo_id) = ctx.todo_id {
            let todo_config = HookDb::get_todo_hook_config(&self.ctx.db.conn, todo_id)
                .await
                .map_err(|e| e.to_string())?;

            if let Some(config) = todo_config {
                match config.hook_mode {
                    HookMode::Disabled => return Ok(vec![]),
                    HookMode::Custom => {
                        // Get custom rule IDs for this todo
                        let custom_rule_ids = HookDb::get_todo_hook_rule_ids(&self.ctx.db.conn, todo_id)
                            .await
                            .map_err(|e| e.to_string())?;

                        if !custom_rule_ids.is_empty() {
                            // Filter rules to only those in custom_rule_ids
                            rules.retain(|r| r.id.map(|id| custom_rule_ids.contains(&id)).unwrap_or(false));
                        }
                    }
                    HookMode::Inherit => {
                        // Use global defaults + trigger-specific hooks
                        let default_ids = HookDb::get_global_default_hook_ids(&self.ctx.db.conn)
                            .await
                            .map_err(|e| e.to_string())?;

                        if !default_ids.is_empty() {
                            rules.retain(|r| r.id.map(|id| default_ids.contains(&id)).unwrap_or(false));
                        }
                    }
                }
            }
        } else {
            // For new todos (no todo_id), use global defaults
            let default_ids = HookDb::get_global_default_hook_ids(&self.ctx.db.conn)
                .await
                .map_err(|e| e.to_string())?;

            if !default_ids.is_empty() {
                rules.retain(|r| r.id.map(|id| default_ids.contains(&id)).unwrap_or(false));
            }
        }

        Ok(rules)
    }

    fn matches_filter(&self, filter: &HookFilter, ctx: &HookContext, tag_ids: &[i64]) -> bool {
        filter.matches(
            &ctx.todo_title,
            ctx.new_status.as_deref().unwrap_or(""),
            tag_ids,
            ctx.executor.as_deref(),
        )
    }

    async fn acquire_permit(&self) -> OwnedSemaphorePermit {
        self.semaphore.clone().acquire_owned().await.expect("semaphore closed")
    }

    async fn execute_hook(&self, rule: &HookRule, ctx: &HookContext) -> HookResult {
        execute_target_todo(
            &self.ctx.db,
            &self.ctx.executor_registry,
            self.ctx.tx.clone(),
            &self.ctx.task_manager,
            &self.ctx.config,
            rule,
            ctx,
        )
        .await
    }
}

/// Trigger the target todo associated with this hook rule.
///
/// Returns a `HookResult` whose `stdout` field carries a JSON summary on success
/// and whose `error_msg` carries the reason on failure.
async fn execute_target_todo(
    db: &Arc<Database>,
    executor_registry: &Arc<crate::adapters::ExecutorRegistry>,
    tx: tokio::sync::broadcast::Sender<crate::handlers::ExecEvent>,
    task_manager: &Arc<crate::task_manager::TaskManager>,
    config: &Arc<tokio::sync::RwLock<crate::config::Config>>,
    rule: &HookRule,
    ctx: &HookContext,
) -> HookResult {
    // 1. Look up the target todo
    let target_todo = match db.get_todo(rule.action.target_todo_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            let msg = format!(
                "Hook '{}' target todo #{} not found",
                rule.name, rule.action.target_todo_id
            );
            if rule.action.skip_if_missing {
                warn!("{}", msg);
                return HookResult::success(
                    0,
                    format!("{{\"skipped\": true, \"reason\": \"target todo not found\"}}"),
                    String::new(),
                    0,
                );
            }
            return HookResult::error(msg, 0);
        }
        Err(e) => {
            return HookResult::error(
                format!("Failed to look up target todo: {}", e),
                0,
            );
        }
    };

    // 2. Determine the message to send to the target todo
    let raw_message = rule
        .action
        .prompt_template
        .clone()
        .unwrap_or_else(|| target_todo.prompt.clone());

    // 3. Build params from hook context
    let mut params = ctx.to_params();
    // Also include the source todo's full context for easy templating
    params.insert("source_todo_id".to_string(), ctx.todo_id.map(|id| id.to_string()).unwrap_or_default());
    params.insert("source_todo_title".to_string(), ctx.todo_title.clone());

    // 4. Trigger the target todo
    //
    // NOTE: We dispatch the call to `start_todo_execution` on a dedicated
    // thread with its own runtime to break an async type-cycle. The path is:
    // run_todo_execution -> fire_before_hooks -> ... -> start_todo_execution
    // -> run_todo_execution. Without runtime-level indirection, Rust
    // computes an infinitely-sized future. Running the call in a fresh
    // runtime preserves the "block the hook caller until the record is
    // created" semantic — we still `await` the oneshot reply, so a sync hook
    // caller still gets the result before returning.
    let trigger_type = format!("hook:{}", ctx.trigger.as_str());
    let request = RunTodoExecutionRequest {
        db: db.clone(),
        executor_registry: executor_registry.clone(),
        tx: tx.clone(),
        task_manager: task_manager.clone(),
        config: config.clone(),
        hook_service: None,
        todo_id: target_todo.id,
        message: raw_message,
        req_executor: target_todo.executor.clone().or(ctx.executor.clone()),
        trigger_type,
        params: Some(params),
        resume_session_id: None,
        resume_message: None,
    };

    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build runtime for hook trigger");
        let result = rt.block_on(start_todo_execution(request));
        let _ = reply_tx.send(result);
    });

    let result = match reply_rx.await {
        Ok(r) => r,
        Err(_) => {
            return HookResult::error(
                format!("Hook trigger thread for todo #{} dropped reply channel", target_todo.id),
                0,
            );
        }
    };

    match result {
        Ok(exec_result) => {
            let summary = serde_json::json!({
                "target_todo_id": target_todo.id,
                "target_todo_title": target_todo.title,
                "task_id": exec_result.task_id,
                "record_id": exec_result.record_id,
            });
            HookResult::success(
                0,
                summary.to_string(),
                String::new(),
                0,
            )
        }
        Err(e) => HookResult::error(
            format!(
                "Failed to trigger target todo #{} '{}': {:?}",
                target_todo.id, target_todo.title, e
            ),
            0,
        ),
    }
}
