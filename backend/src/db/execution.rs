use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Statement,
};

use crate::db::Database;
use crate::db::entity::execution_records;
use crate::models::{ExecutionRecord, ExecutionStatus, ExecutionSummary, ExecutionUsage};

impl From<execution_records::Model> for ExecutionRecord {
    fn from(m: execution_records::Model) -> Self {
        let usage = m.usage.as_deref().and_then(|u| serde_json::from_str(u).ok());
        let execution_stats = m.execution_stats.as_deref().and_then(|s| serde_json::from_str(s).ok());
        let status = m.status.as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                tracing::warn!("Failed to parse execution status, defaulting to Running: {:?}", m.status);
                ExecutionStatus::Running
            });
        ExecutionRecord {
            id: m.id,
            todo_id: m.todo_id.unwrap_or(0),
            status,
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
            task_id: m.task_id,
            session_id: m.session_id,
            todo_progress: m.todo_progress,
            execution_stats,
            resume_message: m.resume_message,
        }
    }
}

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
            .map(Into::into)
            .collect();

        (records, total)
    }

    pub async fn get_execution_record(&self, record_id: i64) -> Option<ExecutionRecord> {
        let m = execution_records::Entity::find()
            .filter(execution_records::Column::Id.eq(record_id))
            .one(&self.conn)
            .await
            .ok()??;
        Some(m.into())
    }

    /// 根据 task_id 获取执行记录
    pub async fn get_execution_record_by_task_id(&self, task_id: &str) -> Option<ExecutionRecord> {
        let m = execution_records::Entity::find()
            .filter(execution_records::Column::TaskId.eq(task_id))
            .one(&self.conn)
            .await
            .ok()??;
        Some(m.into())
    }

    pub async fn create_execution_record(
        &self,
        todo_id: i64,
        command: &str,
        executor: &str,
        trigger_type: &str,
        task_id: &str,
        session_id: Option<&str>,
        resume_message: Option<&str>,
    ) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = execution_records::ActiveModel {
            todo_id: ActiveValue::Set(Some(todo_id)),
            command: ActiveValue::Set(Some(command.to_string())),
            executor: ActiveValue::Set(Some(executor.to_string())),
            trigger_type: ActiveValue::Set(Some(trigger_type.to_string())),
            status: ActiveValue::Set(Some(crate::models::ExecutionStatus::Running.to_string())),
            started_at: ActiveValue::Set(Some(now)),
            task_id: ActiveValue::Set(Some(task_id.to_string())),
            session_id: ActiveValue::Set(session_id.map(|s| s.to_string())),
            resume_message: ActiveValue::Set(resume_message.map(|s| s.to_string())),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await?;
        Ok(inserted.id)
    }

    pub async fn update_execution_record(
        &self,
        id: i64,
        status: &str,
        logs: &str,
        result: &str,
        usage: Option<&ExecutionUsage>,
        model: Option<&str>,
    ) -> Result<(), sea_orm::DbErr> {
        // Merge new logs with existing logs in DB (since periodic flush may have drained in-memory vec)
        let existing: Option<String> = execution_records::Entity::find_by_id(id)
            .one(&self.conn)
            .await?
            .and_then(|r| r.logs);

        let merged_logs = match existing {
            Some(ref json) if !json.is_empty() && json != "[]" => {
                let mut base: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
                let append: Vec<serde_json::Value> = serde_json::from_str(logs).unwrap_or_default();
                base.extend(append);
                serde_json::to_string(&base).unwrap_or_else(|_| logs.to_string())
            }
            _ => logs.to_string(),
        };

        let now = crate::models::utc_timestamp();
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            logs: ActiveValue::Set(Some(merged_logs)),
            result: ActiveValue::Set(Some(result.to_string())),
            usage: ActiveValue::Set(usage.map(|u| serde_json::to_string(u).unwrap_or_else(|e| { tracing::error!("Failed to serialize usage: {}", e); String::new() }))),
            model: ActiveValue::Set(model.map(|s| s.to_string())),
            finished_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        self.exec_update(am).await
    }

    /// 更新执行记录的 pid
    pub async fn update_execution_record_pid(&self, id: i64, pid: Option<i32>) -> Result<(), sea_orm::DbErr> {
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            pid: ActiveValue::Set(pid),
            ..Default::default()
        };
        self.exec_update(am).await
    }

    /// 更新执行记录的 session_id
    pub async fn update_execution_record_session_id(&self, id: i64, session_id: &str) -> Result<(), sea_orm::DbErr> {
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            session_id: ActiveValue::Set(Some(session_id.to_string())),
            ..Default::default()
        };
        self.exec_update(am).await
    }

    /// 更新执行记录的 todo_progress
    pub async fn update_execution_record_todo_progress(&self, id: i64, todo_progress_json: &str) -> Result<(), sea_orm::DbErr> {
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            todo_progress: ActiveValue::Set(Some(todo_progress_json.to_string())),
            ..Default::default()
        };
        self.exec_update(am).await
    }

    /// 更新执行记录的 execution_stats
    pub async fn update_execution_record_stats(&self, id: i64, stats_json: &str) -> Result<(), sea_orm::DbErr> {
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            execution_stats: ActiveValue::Set(Some(stats_json.to_string())),
            ..Default::default()
        };
        self.exec_update(am).await
    }

    /// 追加日志条目到执行记录（读取已有日志 + 合并新条目 + 写回，防止覆盖）
    pub async fn append_execution_record_logs(&self, id: i64, new_logs_json: &str) -> Result<(), sea_orm::DbErr> {
        let existing: Option<String> = execution_records::Entity::find_by_id(id)
            .one(&self.conn)
            .await?
            .and_then(|r| r.logs);

        let merged = match existing {
            Some(ref json) if !json.is_empty() && json != "[]" => {
                let mut base: Vec<serde_json::Value> = match serde_json::from_str(json) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!("Failed to parse existing logs JSON for record {}: {}", id, e);
                        Vec::new()
                    }
                };
                let append: Vec<serde_json::Value> = match serde_json::from_str(new_logs_json) {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::warn!("Failed to parse new logs JSON for record {}: {}", id, e);
                        Vec::new()
                    }
                };
                base.extend(append);
                serde_json::to_string(&base).unwrap_or_else(|_| new_logs_json.to_string())
            }
            _ => new_logs_json.to_string(),
        };

        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            logs: ActiveValue::Set(Some(merged)),
            ..Default::default()
        };
        self.exec_update(am).await
    }

    /// 根据 session_id 获取所有执行记录（按 started_at 排序）
    pub async fn get_execution_records_by_session(&self, session_id: &str) -> Vec<ExecutionRecord> {
        execution_records::Entity::find()
            .filter(execution_records::Column::SessionId.eq(session_id))
            .order_by_asc(execution_records::Column::StartedAt)
            .all(&self.conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect()
    }

    pub async fn get_dashboard_stats(&self) -> crate::models::DashboardStats {
        use std::collections::HashMap;

        let backend = self.conn.get_database_backend();

        // 1. Todo status counts via SQL (replaces get_todos() + in-memory filtering)
        let todo_sql = "SELECT \
            COUNT(*) as total, \
            COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) as pending, \
            COALESCE(SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END), 0) as running, \
            COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0) as completed, \
            COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) as failed, \
            COALESCE(SUM(CASE WHEN scheduler_enabled = 1 AND scheduler_config IS NOT NULL THEN 1 ELSE 0 END), 0) as scheduled \
            FROM todos WHERE deleted_at IS NULL";

        let (total_todos, pending_todos, running_todos, completed_todos, failed_todos, scheduled_todos) =
            if let Ok(Some(row)) = self.conn.query_one(Statement::from_string(backend, todo_sql.to_string())).await {
                (
                    row.try_get_by("total").unwrap_or(0i64),
                    row.try_get_by("pending").unwrap_or(0i64),
                    row.try_get_by("running").unwrap_or(0i64),
                    row.try_get_by("completed").unwrap_or(0i64),
                    row.try_get_by("failed").unwrap_or(0i64),
                    row.try_get_by("scheduled").unwrap_or(0i64),
                )
            } else {
                (0i64, 0i64, 0i64, 0i64, 0i64, 0i64)
            };

        let tags = self.get_tags().await.unwrap();
        let total_tags = tags.len() as i64;

        // Executor todo counts via SQL (replaces in-memory iteration over all todos)
        let executor_todo_sql = "SELECT \
            COALESCE(executor, 'claudecode') as executor, \
            COUNT(*) as todo_count \
            FROM todos WHERE deleted_at IS NULL \
            GROUP BY COALESCE(executor, 'claudecode')";

        let executor_todo_counts: HashMap<String, i64> = self.conn
            .query_all(Statement::from_string(backend, executor_todo_sql.to_string()))
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                let exec: String = row.try_get_by("executor").ok()?;
                let count: i64 = row.try_get_by("todo_count").ok()?;
                Some((exec, count))
            })
            .collect();

        // Tag todo counts via SQL (replaces fetch_tag_ids_for_many + in-memory counting)
        let tag_todo_sql = "SELECT tag_id, COUNT(*) as todo_count FROM todo_tags GROUP BY tag_id";

        let tag_todo_counts: HashMap<i64, i64> = self.conn
            .query_all(Statement::from_string(backend, tag_todo_sql.to_string()))
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                let tag_id: i64 = row.try_get_by("tag_id").ok()?;
                let count: i64 = row.try_get_by("todo_count").ok()?;
                Some((tag_id, count))
            })
            .collect();

        // 2. Overall execution stats with token aggregation
        let overall_sql = "SELECT \
            COUNT(*) as total, \
            COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0) as success, \
            COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) as failed, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.input_tokens'), 0)), 0) as input_tokens, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.output_tokens'), 0)), 0) as output_tokens, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.cache_read_input_tokens'), 0)), 0) as cache_read, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.cache_creation_input_tokens'), 0)), 0) as cache_creation, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.total_cost_usd'), 0.0)), 0.0) as total_cost, \
            COALESCE(SUM(CASE WHEN json_extract(usage, '$.duration_ms') IS NOT NULL THEN json_extract(usage, '$.duration_ms') ELSE 0 END), 0) as total_duration, \
            COALESCE(SUM(CASE WHEN json_extract(usage, '$.duration_ms') IS NOT NULL THEN 1 ELSE 0 END), 0) as duration_count \
            FROM execution_records";

        let (total_executions, success_executions, failed_executions,
             total_input_tokens, total_output_tokens, total_cache_read_tokens,
             total_cache_creation_tokens, total_cost, total_duration, duration_count) =
            if let Ok(Some(row)) = self.conn.query_one(Statement::from_string(backend, overall_sql.to_string())).await {
                let t: i64 = row.try_get_by("total").unwrap_or(0);
                let s: i64 = row.try_get_by("success").unwrap_or(0);
                let f: i64 = row.try_get_by("failed").unwrap_or(0);
                let it: i64 = row.try_get_by("input_tokens").unwrap_or(0);
                let ot: i64 = row.try_get_by("output_tokens").unwrap_or(0);
                let cr: i64 = row.try_get_by("cache_read").unwrap_or(0);
                let cc: i64 = row.try_get_by("cache_creation").unwrap_or(0);
                let tc: f64 = row.try_get_by("total_cost").unwrap_or(0.0);
                let td: i64 = row.try_get_by("total_duration").unwrap_or(0);
                let dc: i64 = row.try_get_by("duration_count").unwrap_or(0);
                (t, s, f, it as u64, ot as u64, cr as u64, cc as u64, tc, td as u64, dc as u64)
            } else {
                (0, 0, 0, 0u64, 0u64, 0u64, 0u64, 0.0f64, 0u64, 0u64)
            };

        let avg_duration_ms = if duration_count > 0 { total_duration / duration_count } else { 0 };

        // 3. Executor distribution via SQL
        let executor_sql = "SELECT \
            COALESCE(executor, 'claudecode') as executor, \
            COUNT(*) as execution_count, \
            COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0) as success_count, \
            COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) as failed_count, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.input_tokens'), 0)), 0) as input_tokens, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.output_tokens'), 0)), 0) as output_tokens, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.total_cost_usd'), 0.0)), 0.0) as cost \
            FROM execution_records \
            GROUP BY COALESCE(executor, 'claudecode')";

        let mut executor_distribution: Vec<crate::models::ExecutorCount> = self.conn
            .query_all(Statement::from_string(backend, executor_sql.to_string()))
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                let exec: String = row.try_get_by("executor").ok()?;
                let ec: i64 = row.try_get_by("execution_count").ok()?;
                if ec == 0 { return None; }
                let sc: i64 = row.try_get_by("success_count").ok()?;
                let fc: i64 = row.try_get_by("failed_count").ok()?;
                let it: i64 = row.try_get_by("input_tokens").ok()?;
                let ot: i64 = row.try_get_by("output_tokens").ok()?;
                let cost: f64 = row.try_get_by("cost").ok()?;
                Some(crate::models::ExecutorCount {
                    count: *executor_todo_counts.get(&exec).unwrap_or(&0),
                    executor: exec,
                    execution_count: ec,
                    success_count: sc,
                    failed_count: fc,
                    total_input_tokens: it as u64,
                    total_output_tokens: ot as u64,
                    total_cost_usd: cost,
                })
            })
            .collect();
        executor_distribution.sort_by(|a, b| b.execution_count.cmp(&a.execution_count));

        // 4. Model distribution via SQL
        let model_sql = "SELECT \
            COALESCE(model, 'unknown') as model, \
            COUNT(*) as execution_count, \
            COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0) as success_count, \
            COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) as failed_count, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.input_tokens'), 0)), 0) as input_tokens, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.output_tokens'), 0)), 0) as output_tokens, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.cache_read_input_tokens'), 0)), 0) as cache_read, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.cache_creation_input_tokens'), 0)), 0) as cache_creation, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.total_cost_usd'), 0.0)), 0.0) as cost \
            FROM execution_records \
            GROUP BY COALESCE(model, 'unknown')";

        let mut model_distribution: Vec<crate::models::ModelCount> = self.conn
            .query_all(Statement::from_string(backend, model_sql.to_string()))
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                let model: String = row.try_get_by("model").ok()?;
                let ec: i64 = row.try_get_by("execution_count").ok()?;
                if ec == 0 { return None; }
                let sc: i64 = row.try_get_by("success_count").ok()?;
                let fc: i64 = row.try_get_by("failed_count").ok()?;
                let it: i64 = row.try_get_by("input_tokens").ok()?;
                let ot: i64 = row.try_get_by("output_tokens").ok()?;
                let cr: i64 = row.try_get_by("cache_read").ok()?;
                let cc: i64 = row.try_get_by("cache_creation").ok()?;
                let cost: f64 = row.try_get_by("cost").ok()?;
                Some(crate::models::ModelCount {
                    model,
                    count: 0,
                    execution_count: ec,
                    success_count: sc,
                    failed_count: fc,
                    total_input_tokens: it as u64,
                    total_output_tokens: ot as u64,
                    total_cache_read_tokens: cr as u64,
                    total_cache_creation_tokens: cc as u64,
                    total_cost_usd: cost,
                })
            })
            .collect();
        model_distribution.sort_by(|a, b| b.execution_count.cmp(&a.execution_count));

        // 5. Daily execution stats via SQL
        let daily_sql = "SELECT \
            SUBSTR(COALESCE(started_at, ''), 1, 10) as day, \
            COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0) as success, \
            COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) as failed, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.input_tokens'), 0)), 0) as input_tokens, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.output_tokens'), 0)), 0) as output_tokens, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.cache_read_input_tokens'), 0)), 0) as cache_read, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.cache_creation_input_tokens'), 0)), 0) as cache_creation, \
            COALESCE(SUM(COALESCE(json_extract(usage, '$.total_cost_usd'), 0.0)), 0.0) as cost \
            FROM execution_records \
            WHERE started_at IS NOT NULL AND LENGTH(started_at) >= 10 \
            GROUP BY SUBSTR(started_at, 1, 10) \
            ORDER BY day DESC \
            LIMIT 30";

        let daily_rows = self.conn
            .query_all(Statement::from_string(backend, daily_sql.to_string()))
            .await
            .unwrap_or_default();

        let mut daily_executions: Vec<crate::models::DailyExecution> = Vec::with_capacity(daily_rows.len());
        let mut daily_token_stats: Vec<crate::models::DailyTokenStats> = Vec::with_capacity(daily_rows.len());
        for row in &daily_rows {
            let day: String = row.try_get_by("day").unwrap_or_default();
            let success: i64 = row.try_get_by("success").unwrap_or(0);
            let failed: i64 = row.try_get_by("failed").unwrap_or(0);
            daily_executions.push(crate::models::DailyExecution { date: day.clone(), success, failed });

            let it: i64 = row.try_get_by("input_tokens").unwrap_or(0);
            let ot: i64 = row.try_get_by("output_tokens").unwrap_or(0);
            let cr: i64 = row.try_get_by("cache_read").unwrap_or(0);
            let cc: i64 = row.try_get_by("cache_creation").unwrap_or(0);
            let cost: f64 = row.try_get_by("cost").unwrap_or(0.0);
            daily_token_stats.push(crate::models::DailyTokenStats {
                date: day,
                input_tokens: it as u64,
                output_tokens: ot as u64,
                cache_read_tokens: cr as u64,
                cache_creation_tokens: cc as u64,
                total_cost_usd: cost,
            });
        }
        daily_executions.reverse();
        daily_token_stats.reverse();

        // 6. Tag distribution via SQL (join through todo_tags)
        let tag_sql = "SELECT \
            tt.tag_id, \
            COUNT(*) as execution_count, \
            COALESCE(SUM(CASE WHEN er.status = 'success' THEN 1 ELSE 0 END), 0) as success_count, \
            COALESCE(SUM(CASE WHEN er.status = 'failed' THEN 1 ELSE 0 END), 0) as failed_count, \
            COALESCE(SUM(COALESCE(json_extract(er.usage, '$.input_tokens'), 0)), 0) as input_tokens, \
            COALESCE(SUM(COALESCE(json_extract(er.usage, '$.output_tokens'), 0)), 0) as output_tokens, \
            COALESCE(SUM(COALESCE(json_extract(er.usage, '$.total_cost_usd'), 0.0)), 0.0) as cost \
            FROM execution_records er \
            INNER JOIN todo_tags tt ON tt.todo_id = er.todo_id \
            WHERE er.todo_id IS NOT NULL \
            GROUP BY tt.tag_id";

        let tag_rows = self.conn
            .query_all(Statement::from_string(backend, tag_sql.to_string()))
            .await
            .unwrap_or_default();

        let mut tag_exec_stats: HashMap<i64, (i64, i64, i64, u64, u64, f64)> = HashMap::new();
        for row in tag_rows {
            let tag_id: i64 = row.try_get_by("tag_id").unwrap_or(0);
            let ec: i64 = row.try_get_by("execution_count").unwrap_or(0);
            let sc: i64 = row.try_get_by("success_count").unwrap_or(0);
            let fc: i64 = row.try_get_by("failed_count").unwrap_or(0);
            let it: i64 = row.try_get_by("input_tokens").unwrap_or(0);
            let ot: i64 = row.try_get_by("output_tokens").unwrap_or(0);
            let cost: f64 = row.try_get_by("cost").unwrap_or(0.0);
            tag_exec_stats.insert(tag_id, (ec, sc, fc, it as u64, ot as u64, cost));
        }

        let mut tag_distribution: Vec<crate::models::TagCount> = tags.iter().filter_map(|t| {
            let todo_count = *tag_todo_counts.get(&t.id).unwrap_or(&0);
            if todo_count == 0 { return None; }
            let (ec, sc, fc, it, ot, cost) = tag_exec_stats.get(&t.id).copied().unwrap_or((0, 0, 0, 0, 0, 0.0));
            Some(crate::models::TagCount {
                tag_id: t.id,
                tag_name: t.name.clone(),
                tag_color: t.color.clone(),
                count: todo_count,
                execution_count: ec,
                success_count: sc,
                failed_count: fc,
                total_input_tokens: it,
                total_output_tokens: ot,
                total_cost_usd: cost,
            })
        }).collect();
        tag_distribution.sort_by(|a, b| b.execution_count.cmp(&a.execution_count));

        // 7. Recent executions (only load 10 rows, not the entire table)
        let recent_records = execution_records::Entity::find()
            .order_by_desc(execution_records::Column::StartedAt)
            .limit(10)
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let recent_executions: Vec<crate::models::ExecutionRecord> = recent_records.into_iter()
            .map(Into::into)
            .collect();

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
            model_distribution,
            daily_executions,
            daily_token_stats,
            recent_executions,
        }
    }

    pub async fn get_execution_summary(&self, todo_id: i64) -> ExecutionSummary {
        let backend = self.conn.get_database_backend();
        let sql = "SELECT \
                COUNT(*) as total, \
                COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0) as success_count, \
                COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) as failed_count, \
                COALESCE(SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END), 0) as running_count, \
                COALESCE(SUM(COALESCE(json_extract(usage, '$.input_tokens'), 0)), 0) as input_tokens, \
                COALESCE(SUM(COALESCE(json_extract(usage, '$.output_tokens'), 0)), 0) as output_tokens, \
                COALESCE(SUM(COALESCE(json_extract(usage, '$.cache_read_input_tokens'), 0)), 0) as cache_read, \
                COALESCE(SUM(COALESCE(json_extract(usage, '$.cache_creation_input_tokens'), 0)), 0) as cache_creation, \
                COALESCE(SUM(COALESCE(json_extract(usage, '$.total_cost_usd'), 0.0)), 0.0) as total_cost \
                FROM execution_records WHERE todo_id = $1";

        if let Ok(Some(row)) = self.conn.query_one(Statement::from_sql_and_values(backend, sql, [todo_id.into()])).await {
            let total_executions: i64 = row.try_get_by("total").unwrap_or(0);
            let success_count: i64 = row.try_get_by("success_count").unwrap_or(0);
            let failed_count: i64 = row.try_get_by("failed_count").unwrap_or(0);
            let running_count: i64 = row.try_get_by("running_count").unwrap_or(0);
            let input_tokens: i64 = row.try_get_by("input_tokens").unwrap_or(0);
            let output_tokens: i64 = row.try_get_by("output_tokens").unwrap_or(0);
            let cache_read: i64 = row.try_get_by("cache_read").unwrap_or(0);
            let cache_creation: i64 = row.try_get_by("cache_creation").unwrap_or(0);
            let total_cost: f64 = row.try_get_by("total_cost").unwrap_or(0.0);

            ExecutionSummary {
                todo_id,
                total_executions,
                success_count,
                failed_count,
                running_count,
                total_input_tokens: input_tokens as u64,
                total_output_tokens: output_tokens as u64,
                total_cache_read_tokens: cache_read as u64,
                total_cache_creation_tokens: cache_creation as u64,
                total_cost_usd: if total_cost > 0.0 { Some(total_cost) } else { None },
            }
        } else {
            ExecutionSummary {
                todo_id,
                total_executions: 0,
                success_count: 0,
                failed_count: 0,
                running_count: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_read_tokens: 0,
                total_cache_creation_tokens: 0,
                total_cost_usd: None,
            }
        }
    }

    /// 清理孤儿执行记录：状态为running但todo没有对应task_id的记录
    /// 程序崩溃后，执行记录可能保持running状态，需要修复
    pub async fn cleanup_orphan_execution_records(&self) {
        let now = crate::models::utc_timestamp();
        let backend = self.conn.get_database_backend();
        let sql = "UPDATE execution_records SET \
                status = 'failed', \
                finished_at = $1, \
                result = CASE \
                    WHEN todo_id NOT IN (SELECT id FROM todos WHERE deleted_at IS NULL) THEN '任务已被删除' \
                    ELSE '程序崩溃，任务被中断' \
                END \
                WHERE status = 'running' AND ( \
                    todo_id NOT IN (SELECT id FROM todos WHERE deleted_at IS NULL) \
                    OR todo_id IN (SELECT id FROM todos WHERE task_id IS NULL AND deleted_at IS NULL) \
                )";
        match self.conn.execute(Statement::from_sql_and_values(backend, sql, [now.into()])).await {
            Ok(res) => {
                let rows = res.rows_affected();
                if rows > 0 {
                    tracing::info!("Cleaned up {} orphan execution records", rows);
                }
            }
            Err(e) => {
                tracing::error!("Failed to cleanup orphan execution records: {}", e);
            }
        }
    }

}
