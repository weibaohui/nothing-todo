use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::db::Database;
use crate::db::entity::execution_records;
use crate::models::{ExecutionRecord, ExecutionSummary, ExecutionUsage};

impl Database {
    pub async fn get_execution_records(
        &self,
        todo_id: i64,
        limit: i64,
        offset: i64,
    ) -> (Vec<ExecutionRecord>, i64) {
        let total: i64 = execution_records::Entity::find()
            .filter(execution_records::Column::TodoId.eq(todo_id))
            .count(&self.conn)
            .await
            .unwrap_or(0) as i64;

        let limit_u = if limit < 0 { 0 } else { limit as u64 };
        let offset_u = if offset < 0 { 0 } else { offset as u64 };

        let records = execution_records::Entity::find()
            .filter(execution_records::Column::TodoId.eq(todo_id))
            .order_by_desc(execution_records::Column::StartedAt)
            .limit(limit_u)
            .offset(offset_u)
            .all(&self.conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| {
                let usage = m
                    .usage
                    .as_deref()
                    .and_then(|u| serde_json::from_str(u).ok());
                ExecutionRecord {
                    id: m.id,
                    todo_id: m.todo_id.unwrap_or(0),
                    status: m.status.unwrap_or_default(),
                    command: m.command.unwrap_or_default(),
                    stdout: m.stdout.unwrap_or_default(),
                    stderr: m.stderr.unwrap_or_default(),
                    logs: m.logs.unwrap_or_default(),
                    result: m.result,
                    started_at: m.started_at.unwrap_or_default(),
                    finished_at: m.finished_at,
                    usage,
                    executor: m.executor,
                    model: m.model,
                    trigger_type: m.trigger_type.unwrap_or_else(|| "manual".to_string()),
                    pid: m.pid,
                }
            })
            .collect();

        (records, total)
    }

    pub async fn create_execution_record(
        &self,
        todo_id: i64,
        command: &str,
        executor: &str,
        trigger_type: &str,
    ) -> i64 {
        let now = crate::models::utc_timestamp();
        let am = execution_records::ActiveModel {
            todo_id: ActiveValue::Set(Some(todo_id)),
            command: ActiveValue::Set(Some(command.to_string())),
            executor: ActiveValue::Set(Some(executor.to_string())),
            trigger_type: ActiveValue::Set(Some(trigger_type.to_string())),
            status: ActiveValue::Set(Some(crate::models::ExecutionStatus::Running.to_string())),
            started_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am
            .insert(&self.conn)
            .await
            .expect("insert execution record failed");
        inserted.id
    }

    pub async fn update_execution_record(
        &self,
        id: i64,
        status: &str,
        logs: &str,
        result: &str,
        usage: Option<&ExecutionUsage>,
        model: Option<&str>,
    ) {
        let now = crate::models::utc_timestamp();
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            logs: ActiveValue::Set(Some(logs.to_string())),
            result: ActiveValue::Set(Some(result.to_string())),
            usage: ActiveValue::Set(usage.map(|u| serde_json::to_string(u).unwrap_or_default())),
            model: ActiveValue::Set(model.map(|s| s.to_string())),
            finished_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    /// 更新执行记录的 pid
    pub async fn update_execution_record_pid(&self, id: i64, pid: Option<i32>) {
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            pid: ActiveValue::Set(pid),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    /// 根据 pid 获取执行记录
    pub async fn get_execution_record_by_pid(&self, pid: i32) -> Option<execution_records::Model> {
        execution_records::Entity::find()
            .filter(execution_records::Column::Pid.eq(pid))
            .one(&self.conn)
            .await
            .unwrap_or_default()
    }

    /// 根据 pid 停止执行记录
    pub async fn stop_execution_by_pid(&self, pid: i32) -> bool {
        if let Some(record) = self.get_execution_record_by_pid(pid).await {
            // 只更新这一条执行记录，不影响 todo 的状态
            let now = crate::models::utc_timestamp();
            let am = execution_records::ActiveModel {
                id: ActiveValue::Unchanged(record.id),
                status: ActiveValue::Set(Some(crate::models::ExecutionStatus::Failed.as_str().to_string())),
                finished_at: ActiveValue::Set(Some(now)),
                result: ActiveValue::Set(Some("任务已被手动停止".to_string())),
                pid: ActiveValue::Set(None),
                ..Default::default()
            };
            self.exec_update(am).await;

            tracing::info!("Stopped execution record {} with pid {}", record.id, pid);
            return true;
        }
        false
    }

    pub async fn get_dashboard_stats(&self) -> crate::models::DashboardStats {
        use std::collections::HashMap;

        let todos = self.get_todos().await;
        let total_todos = todos.len() as i64;
        let pending_todos = todos.iter().filter(|t| t.status == crate::models::TodoStatus::Pending).count() as i64;
        let running_todos = todos.iter().filter(|t| t.status == crate::models::TodoStatus::Running).count() as i64;
        let completed_todos = todos.iter().filter(|t| t.status == crate::models::TodoStatus::Completed).count() as i64;
        let failed_todos = todos.iter().filter(|t| t.status == crate::models::TodoStatus::Failed).count() as i64;
        let scheduled_todos = todos.iter().filter(|t| t.scheduler_enabled && t.scheduler_config.is_some()).count() as i64;

        let tags = self.get_tags().await;
        let total_tags = tags.len() as i64;

        let todo_ids: Vec<i64> = todos.iter().map(|t| t.id).collect();
        let tag_map = self.fetch_tag_ids_for_many(&todo_ids).await;

        // Build executor and tag stat templates from todos
        let mut executor_stats: HashMap<String, crate::models::ExecutorCount> = HashMap::new();
        for t in &todos {
            let exec = t.executor.as_deref().unwrap_or("claudecode");
            executor_stats.entry(exec.to_string()).or_insert_with(|| crate::models::ExecutorCount {
                executor: exec.to_string(),
                count: 0,
                execution_count: 0,
                success_count: 0,
                failed_count: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cost_usd: 0.0,
            }).count += 1;
        }

        let mut tag_stats: HashMap<i64, crate::models::TagCount> = HashMap::new();
        for t in &tags {
            tag_stats.insert(t.id, crate::models::TagCount {
                tag_id: t.id,
                tag_name: t.name.clone(),
                tag_color: t.color.clone(),
                count: 0,
                execution_count: 0,
                success_count: 0,
                failed_count: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cost_usd: 0.0,
            });
        }
        for (_, tids) in &tag_map {
            for tid in tids {
                if let Some(stat) = tag_stats.get_mut(tid) {
                    stat.count += 1;
                }
            }
        }

        let all_records = execution_records::Entity::find()
            .order_by_desc(execution_records::Column::StartedAt)
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let mut total_executions = 0i64;
        let mut success_executions = 0i64;
        let mut failed_executions = 0i64;
        let mut total_input_tokens = 0u64;
        let mut total_output_tokens = 0u64;
        let mut total_cache_read_tokens = 0u64;
        let mut total_cache_creation_tokens = 0u64;
        let mut total_cost = 0.0f64;
        let mut total_duration: u64 = 0;
        let mut duration_count: u64 = 0;
        let mut daily_map: HashMap<String, (i64, i64)> = HashMap::new();

        for r in &all_records {
            total_executions += 1;
            let rec_status = r.status.as_deref();
            match rec_status {
                Some("success") => success_executions += 1,
                Some("failed") => failed_executions += 1,
                _ => {}
            }

            if let Some(usage_str) = &r.usage {
                if let Ok(usage) = serde_json::from_str::<crate::models::ExecutionUsage>(usage_str) {
                    total_input_tokens += usage.input_tokens;
                    total_output_tokens += usage.output_tokens;
                    total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
                    total_cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
                    if let Some(cost) = usage.total_cost_usd {
                        total_cost += cost;
                    }
                    if let Some(dur) = usage.duration_ms {
                        total_duration += dur;
                        duration_count += 1;
                    }
                }
            }

            // Aggregate by executor
            if let Some(exec) = &r.executor {
                if let Some(stat) = executor_stats.get_mut(exec) {
                    stat.execution_count += 1;
                    match rec_status {
                        Some("success") => stat.success_count += 1,
                        Some("failed") => stat.failed_count += 1,
                        _ => {}
                    }
                    if let Some(usage_str) = &r.usage {
                        if let Ok(usage) = serde_json::from_str::<crate::models::ExecutionUsage>(usage_str) {
                            stat.total_input_tokens += usage.input_tokens;
                            stat.total_output_tokens += usage.output_tokens;
                            if let Some(cost) = usage.total_cost_usd {
                                stat.total_cost_usd += cost;
                            }
                        }
                    }
                }
            }

            // Aggregate by tag
            let rec_todo_id = r.todo_id.unwrap_or(0);
            if let Some(tids) = tag_map.get(&rec_todo_id) {
                for tid in tids {
                    if let Some(stat) = tag_stats.get_mut(tid) {
                        stat.execution_count += 1;
                        match rec_status {
                            Some("success") => stat.success_count += 1,
                            Some("failed") => stat.failed_count += 1,
                            _ => {}
                        }
                        if let Some(usage_str) = &r.usage {
                            if let Ok(usage) = serde_json::from_str::<crate::models::ExecutionUsage>(usage_str) {
                                stat.total_input_tokens += usage.input_tokens;
                                stat.total_output_tokens += usage.output_tokens;
                                if let Some(cost) = usage.total_cost_usd {
                                    stat.total_cost_usd += cost;
                                }
                            }
                        }
                    }
                }
            }

            if let Some(date) = r.started_at.as_deref() {
                if date.len() >= 10 {
                    let day = date[..10].to_string();
                    let entry = daily_map.entry(day).or_insert((0, 0));
                    match rec_status {
                        Some("success") => entry.0 += 1,
                        Some("failed") => entry.1 += 1,
                        _ => {}
                    }
                }
            }
        }

        let mut executor_distribution: Vec<crate::models::ExecutorCount> = executor_stats
            .into_values()
            .filter(|s| s.count > 0)
            .collect();
        executor_distribution.sort_by(|a, b| b.execution_count.cmp(&a.execution_count));

        let mut tag_distribution: Vec<crate::models::TagCount> = tag_stats
            .into_values()
            .filter(|s| s.count > 0)
            .collect();
        tag_distribution.sort_by(|a, b| b.execution_count.cmp(&a.execution_count));

        let mut daily_executions: Vec<crate::models::DailyExecution> = daily_map.into_iter()
            .map(|(date, (success, failed))| crate::models::DailyExecution { date, success, failed })
            .collect();
        daily_executions.sort_by(|a, b| a.date.cmp(&b.date));
        if daily_executions.len() > 30 {
            daily_executions = daily_executions.into_iter().rev().take(30).collect();
            daily_executions.reverse();
        }

        let recent_executions: Vec<crate::models::ExecutionRecord> = all_records.into_iter()
            .take(10)
            .map(|m| {
                let usage = m.usage.as_deref().and_then(|u| serde_json::from_str(u).ok());
                crate::models::ExecutionRecord {
                    id: m.id,
                    todo_id: m.todo_id.unwrap_or(0),
                    status: m.status.unwrap_or_default(),
                    command: m.command.unwrap_or_default(),
                    stdout: m.stdout.unwrap_or_default(),
                    stderr: m.stderr.unwrap_or_default(),
                    logs: m.logs.unwrap_or_default(),
                    result: m.result,
                    started_at: m.started_at.unwrap_or_default(),
                    finished_at: m.finished_at,
                    usage,
                    executor: m.executor,
                    model: m.model,
                    trigger_type: m.trigger_type.unwrap_or_else(|| "manual".to_string()),
                    pid: m.pid,
                }
            })
            .collect();

        let avg_duration_ms = if duration_count > 0 { total_duration / duration_count } else { 0 };

        crate::models::DashboardStats {
            total_todos,
            pending_todos,
            running_todos,
            completed_todos,
            failed_todos,
            total_tags,
            scheduled_todos,
            total_executions,
            success_executions,
            failed_executions,
            total_input_tokens,
            total_output_tokens,
            total_cache_read_tokens,
            total_cache_creation_tokens,
            total_cost_usd: total_cost,
            avg_duration_ms,
            executor_distribution,
            tag_distribution,
            daily_executions,
            recent_executions,
        }
    }

    pub async fn get_execution_summary(&self, todo_id: i64) -> ExecutionSummary {
        let records = execution_records::Entity::find()
            .filter(execution_records::Column::TodoId.eq(todo_id))
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let mut total_executions = 0i64;
        let mut success_count = 0i64;
        let mut failed_count = 0i64;
        let mut running_count = 0i64;
        let mut total_input_tokens = 0u64;
        let mut total_output_tokens = 0u64;
        let mut total_cache_read_tokens = 0u64;
        let mut total_cache_creation_tokens = 0u64;
        let mut total_cost = 0.0f64;

        for r in records {
            total_executions += 1;
            match r.status.as_deref() {
                Some(s) if s == crate::models::ExecutionStatus::Success.as_str() => success_count += 1,
                Some(s) if s == crate::models::ExecutionStatus::Failed.as_str() => failed_count += 1,
                Some(s) if s == crate::models::ExecutionStatus::Running.as_str() => running_count += 1,
                _ => {}
            }
            if let Some(usage_str) = r.usage {
                if let Ok(usage) = serde_json::from_str::<ExecutionUsage>(&usage_str) {
                    total_input_tokens += usage.input_tokens;
                    total_output_tokens += usage.output_tokens;
                    total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
                    total_cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
                    if let Some(cost) = usage.total_cost_usd {
                        total_cost += cost;
                    }
                }
            }
        }

        ExecutionSummary {
            todo_id,
            total_executions,
            success_count,
            failed_count,
            running_count,
            total_input_tokens,
            total_output_tokens,
            total_cache_read_tokens,
            total_cache_creation_tokens,
            total_cost_usd: if total_cost > 0.0 { Some(total_cost) } else { None },
        }
    }

    /// 清理孤儿执行记录：状态为running但todo没有对应task_id的记录
    /// 程序崩溃后，执行记录可能保持running状态，需要修复
    pub async fn cleanup_orphan_execution_records(&self) {
        // 查找所有状态为running的执行记录
        let running_records = execution_records::Entity::find()
            .filter(execution_records::Column::Status.eq(crate::models::ExecutionStatus::Running.as_str()))
            .all(&self.conn)
            .await
            .unwrap_or_default();

        for record in running_records {
            // 检查对应的todo是否有task_id
            if let Some(todo) = self.get_todo(record.todo_id.unwrap_or(0)).await {
                if todo.task_id.is_none() {
                    // todo没有task_id，说明这个执行记录是孤儿的，标记为失败
                    tracing::warn!(
                        "Found orphan execution record {} for todo {}, marking as failed",
                        record.id,
                        record.todo_id.unwrap_or(0)
                    );
                    let now = crate::models::utc_timestamp();
                    let am = execution_records::ActiveModel {
                        id: ActiveValue::Unchanged(record.id),
                        status: ActiveValue::Set(Some(crate::models::ExecutionStatus::Failed.as_str().to_string())),
                        finished_at: ActiveValue::Set(Some(now)),
                        result: ActiveValue::Set(Some("程序崩溃，任务被中断".to_string())),
                        ..Default::default()
                    };
                    self.exec_update(am).await;
                }
            } else {
                // todo不存在，直接标记执行记录为失败
                tracing::warn!(
                    "Found orphan execution record {} for non-existent todo {}, marking as failed",
                    record.id,
                    record.todo_id.unwrap_or(0)
                );
                let now = crate::models::utc_timestamp();
                let am = execution_records::ActiveModel {
                    id: ActiveValue::Unchanged(record.id),
                    status: ActiveValue::Set(Some(crate::models::ExecutionStatus::Failed.as_str().to_string())),
                    finished_at: ActiveValue::Set(Some(now)),
                    result: ActiveValue::Set(Some("任务已被删除".to_string())),
                    ..Default::default()
                };
                self.exec_update(am).await;
            }
        }
    }

    /// 标记指定todo的所有running状态执行记录为failed
    pub async fn mark_execution_records_as_failed(&self, todo_id: i64) {
        let running_records = execution_records::Entity::find()
            .filter(execution_records::Column::TodoId.eq(todo_id))
            .filter(execution_records::Column::Status.eq(crate::models::ExecutionStatus::Running.as_str()))
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let count = running_records.len();
        if count > 0 {
            tracing::warn!(
                "Marking {} running execution records as failed for todo {}",
                count,
                todo_id
            );

            let now = crate::models::utc_timestamp();
            for record in running_records {
                let am = execution_records::ActiveModel {
                    id: ActiveValue::Unchanged(record.id),
                    status: ActiveValue::Set(Some(crate::models::ExecutionStatus::Failed.as_str().to_string())),
                    finished_at: ActiveValue::Set(Some(now.clone())),
                    result: ActiveValue::Set(Some("任务已被手动停止".to_string())),
                    ..Default::default()
                };
                self.exec_update(am).await;
            }

            tracing::info!(
                "Successfully marked {} execution records as failed for todo {}",
                count,
                todo_id
            );
        }
    }
}
