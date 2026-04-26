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
        let now = super::now_utc();
        let am = execution_records::ActiveModel {
            todo_id: ActiveValue::Set(Some(todo_id)),
            command: ActiveValue::Set(Some(command.to_string())),
            executor: ActiveValue::Set(Some(executor.to_string())),
            trigger_type: ActiveValue::Set(Some(trigger_type.to_string())),
            status: ActiveValue::Set(Some("running".to_string())),
            started_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am
            .insert(&self.conn)
            .await
            .expect("insert execution record failed");
        inserted.id
    }

    pub async fn update_execution_record(&self, id: i64, status: &str, logs: &str, result: &str) {
        let now = super::now_utc();
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            logs: ActiveValue::Set(Some(logs.to_string())),
            result: ActiveValue::Set(Some(result.to_string())),
            finished_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_execution_record_with_usage(
        &self,
        id: i64,
        status: &str,
        logs: &str,
        result: &str,
        usage: &ExecutionUsage,
    ) {
        let now = super::now_utc();
        let usage_json = serde_json::to_string(usage).unwrap_or_default();
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            logs: ActiveValue::Set(Some(logs.to_string())),
            result: ActiveValue::Set(Some(result.to_string())),
            usage: ActiveValue::Set(Some(usage_json)),
            finished_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_execution_record_with_model(&self, id: i64, model: &str) {
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            model: ActiveValue::Set(Some(model.to_string())),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
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
                Some("success") => success_count += 1,
                Some("failed") => failed_count += 1,
                Some("running") => running_count += 1,
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
}
