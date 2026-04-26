use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};

use crate::db::Database;
use crate::db::entity::{todo_tags, todos};
use crate::models::{Todo, TodoStatus};

impl Database {
    fn model_to_todo(m: todos::Model, tag_ids: Vec<i64>) -> Todo {
        let scheduler_enabled = m.scheduler_enabled.unwrap_or(false);
        let scheduler_config = m.scheduler_config.clone();
        let scheduler_next_run_at = if scheduler_enabled {
            scheduler_config.as_deref().and_then(super::compute_next_run)
        } else {
            None
        };
        Todo {
            id: m.id,
            title: m.title,
            prompt: m.prompt.unwrap_or_default(),
            status: m.status.as_deref().and_then(|s| s.parse().ok()).unwrap_or(TodoStatus::Pending),
            created_at: m.created_at.unwrap_or_default(),
            updated_at: m.updated_at.unwrap_or_default(),
            tag_ids,
            executor: m.executor,
            scheduler_enabled,
            scheduler_config,
            scheduler_next_run_at,
            task_id: m.task_id,
        }
    }

    pub(crate) async fn fetch_tag_ids_for_many(&self, todo_ids: &[i64]) -> std::collections::HashMap<i64, Vec<i64>> {
        if todo_ids.is_empty() {
            return std::collections::HashMap::new();
        }
        todo_tags::Entity::find()
            .filter(todo_tags::Column::TodoId.is_in(todo_ids.to_vec()))
            .all(&self.conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .fold(std::collections::HashMap::new(), |mut map, t| {
                map.entry(t.todo_id).or_default().push(t.tag_id);
                map
            })
    }

    pub async fn get_todos(&self) -> Vec<Todo> {
        let models = todos::Entity::find()
            .filter(todos::Column::DeletedAt.is_null())
            .order_by_desc(todos::Column::UpdatedAt)
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let ids: Vec<i64> = models.iter().map(|m| m.id).collect();
        let tag_map = self.fetch_tag_ids_for_many(&ids).await;

        models
            .into_iter()
            .map(|m| {
                let tag_ids = tag_map.get(&m.id).cloned().unwrap_or_default();
                Self::model_to_todo(m, tag_ids)
            })
            .collect()
    }

    pub async fn create_todo(&self, title: &str, prompt: &str) -> i64 {
        let now = crate::models::utc_timestamp();
        let am = todos::ActiveModel {
            title: ActiveValue::Set(title.to_string()),
            prompt: ActiveValue::Set(Some(prompt.to_string())),
            status: ActiveValue::Set(Some(TodoStatus::Pending.to_string())),
            created_at: ActiveValue::Set(Some(now.clone())),
            updated_at: ActiveValue::Set(Some(now)),
            executor: ActiveValue::Set(Some("claudecode".to_string())),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await.expect("insert todo failed");
        inserted.id
    }

    pub async fn update_todo_full(
        &self,
        id: i64,
        title: &str,
        prompt: &str,
        status: TodoStatus,
        executor: Option<&str>,
        scheduler_enabled: Option<bool>,
        scheduler_config: Option<&str>,
    ) {
        let now = crate::models::utc_timestamp();
        let mut am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            title: ActiveValue::Set(title.to_string()),
            prompt: ActiveValue::Set(Some(prompt.to_string())),
            status: ActiveValue::Set(Some(status.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        if let Some(exec) = executor {
            am.executor = ActiveValue::Set(Some(exec.to_string()));
        }
        if let Some(enabled) = scheduler_enabled {
            am.scheduler_enabled = ActiveValue::Set(Some(enabled));
        }
        if let Some(cfg) = scheduler_config {
            am.scheduler_config = ActiveValue::Set(Some(cfg.to_string()));
        }
        self.exec_update(am).await;
    }

    pub async fn update_todo_executor(&self, id: i64, executor: &str) {
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            executor: ActiveValue::Set(Some(executor.to_string())),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    pub async fn update_todo_task_id(&self, id: i64, task_id: Option<&str>) {
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            task_id: ActiveValue::Set(task_id.map(|s| s.to_string())),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    pub async fn update_todo_scheduler(&self, id: i64, enabled: bool, config: Option<&str>) {
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            scheduler_enabled: ActiveValue::Set(Some(enabled)),
            scheduler_config: ActiveValue::Set(config.map(|s| s.to_string())),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    pub async fn force_update_todo_status(&self,
        id: i64,
        status: TodoStatus,
    ) {
        let now = crate::models::utc_timestamp();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    pub async fn delete_todo(&self, id: i64) {
        let now = crate::models::utc_timestamp();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            deleted_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    pub async fn get_todo(&self, id: i64) -> Option<Todo> {
        let model = todos::Entity::find_by_id(id)
            .filter(todos::Column::DeletedAt.is_null())
            .one(&self.conn)
            .await
            .ok()
            .flatten()?;
        let tag_ids = todo_tags::Entity::find()
            .filter(todo_tags::Column::TodoId.eq(id))
            .all(&self.conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|t| t.tag_id)
            .collect();
        Some(Self::model_to_todo(model, tag_ids))
    }

    pub async fn get_scheduler_todos(&self) -> Vec<Todo> {
        let models = todos::Entity::find()
            .filter(todos::Column::DeletedAt.is_null())
            .filter(todos::Column::SchedulerConfig.is_not_null())
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let ids: Vec<i64> = models.iter().map(|m| m.id).collect();
        let tag_map = self.fetch_tag_ids_for_many(&ids).await;

        models
            .into_iter()
            .map(|m| {
                let tag_ids = tag_map.get(&m.id).cloned().unwrap_or_default();
                Self::model_to_todo(m, tag_ids)
            })
            .collect()
    }

    pub async fn get_running_todos(&self) -> Vec<Todo> {
        let models = todos::Entity::find()
            .filter(todos::Column::DeletedAt.is_null())
            .filter(todos::Column::Status.eq(TodoStatus::Running.to_string()))
            .filter(todos::Column::TaskId.is_not_null())
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let ids: Vec<i64> = models.iter().map(|m| m.id).collect();
        let tag_map = self.fetch_tag_ids_for_many(&ids).await;

        models
            .into_iter()
            .map(|m| {
                let tag_ids = tag_map.get(&m.id).cloned().unwrap_or_default();
                Self::model_to_todo(m, tag_ids)
            })
            .collect()
    }

    pub async fn update_todo_status(&self, todo_id: i64, status: TodoStatus) {
        let now = crate::models::utc_timestamp();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(todo_id),
            status: ActiveValue::Set(Some(status.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    pub async fn start_todo_execution(&self, todo_id: i64, task_id: &str) {
        let now = crate::models::utc_timestamp();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(todo_id),
            status: ActiveValue::Set(Some(TodoStatus::Running.to_string())),
            task_id: ActiveValue::Set(Some(task_id.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    pub async fn finish_todo_execution(&self, todo_id: i64, success: bool) {
        let status = if success { TodoStatus::Completed } else { TodoStatus::Failed };
        let now = crate::models::utc_timestamp();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(todo_id),
            status: ActiveValue::Set(Some(status.to_string())),
            task_id: ActiveValue::Set(None),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        self.exec_update(am).await;
    }

    /// 根据task_id查找对应的todo
    pub async fn get_todo_by_task_id(&self, task_id: &str) -> Option<Todo> {
        let model = todos::Entity::find()
            .filter(todos::Column::TaskId.eq(task_id))
            .filter(todos::Column::DeletedAt.is_null())
            .one(&self.conn)
            .await
            .unwrap_or(None)?;

        let tag_map = self.fetch_tag_ids_for_many(&[model.id]).await;
        let tag_ids = tag_map.get(&model.id).cloned().unwrap_or_default();
        Some(Self::model_to_todo(model, tag_ids))
    }
}
