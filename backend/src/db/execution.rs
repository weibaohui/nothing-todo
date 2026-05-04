use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Statement,
};

use crate::db::Database;
use crate::db::entity::execution_records;
use crate::models::{ExecutionRecord, ExecutionSummary, ExecutionUsage};

impl From<execution_records::Model> for ExecutionRecord {
    fn from(m: execution_records::Model) -> Self {
        let usage = m.usage.as_deref().and_then(|u| serde_json::from_str(u).ok());
        let execution_stats = m.execution_stats.as_deref().and_then(|s| serde_json::from_str(s).ok());
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
        let now = crate::models::utc_timestamp();
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            logs: ActiveValue::Set(Some(logs.to_string())),
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

    /// 更新执行记录的 logs 字段（定时批量写入，防止崩溃丢失）
    pub async fn update_execution_record_logs(&self, id: i64, logs_json: &str) -> Result<(), sea_orm::DbErr> {
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            logs: ActiveValue::Set(Some(logs_json.to_string())),
            ..Default::default()
        };
        self.exec_update(am).await
    }

    /// 根据 pid 获取执行记录
    pub async fn get_execution_record_by_pid(&self, pid: i32) -> Option<execution_records::Model> {
        execution_records::Entity::find()
            .filter(execution_records::Column::Pid.eq(pid))
            .one(&self.conn)
            .await
            .unwrap_or_default()
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

    /// 根据 pid 停止执行记录
    pub async fn stop_execution_by_pid(&self, pid: i32) -> Result<bool, sea_orm::DbErr> {
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
            self.exec_update(am).await?;

            tracing::info!("Stopped execution record {} with pid {}", record.id, pid);
            return Ok(true);
        }
        Ok(false)
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
        let mut executor_todo_counts: HashMap<String, i64> = HashMap::new();
        for t in &todos {
            let exec = t.executor.as_deref().unwrap_or("claudecode").to_string();
            *executor_todo_counts.entry(exec).or_insert(0) += 1;
        }

        let mut tag_todo_counts: HashMap<i64, i64> = HashMap::new();
        for (_, tids) in &tag_map {
            for tid in tids {
                *tag_todo_counts.entry(*tid).or_insert(0) += 1;
            }
        }

        // SQL-based aggregation for execution records (avoids loading all records into memory)
        let backend = self.conn.get_database_backend();

        // 1. Overall execution stats with token aggregation
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

        // 2. Executor distribution via SQL
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

        // 3. Model distribution via SQL
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

        // 4. Daily execution stats via SQL
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

        // 5. Tag distribution via SQL (join through todo_tags)
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

        // 6. Recent executions (only load 10 rows, not the entire table)
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

    /// 标记指定todo的所有running状态执行记录为failed
    pub async fn mark_execution_records_as_failed(&self, todo_id: i64) {
        let now = crate::models::utc_timestamp();
        let backend = self.conn.get_database_backend();
        let sql = "UPDATE execution_records SET \
                status = 'failed', \
                finished_at = $1, \
                result = '任务已被手动停止' \
                WHERE todo_id = $2 AND status = 'running'";
        let result = self.conn.execute(Statement::from_sql_and_values(backend, sql, [now.into(), todo_id.into()])).await;
        match result {
            Ok(res) => {
                let rows = res.rows_affected();
                if rows > 0 {
                    tracing::warn!(
                        "Marked {} running execution records as failed for todo {}",
                        rows,
                        todo_id
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to mark execution records as failed for todo {}: {}", todo_id, e);
            }
        }
    }
}
