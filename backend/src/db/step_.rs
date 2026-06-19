use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use crate::db::entity::steps;
use crate::db::Database;

impl Database {
    /// 列出所有环节（按 id 倒序）。
    pub async fn list_steps_pure(&self) -> Result<Vec<steps::Model>, sea_orm::DbErr> {
        steps::Entity::find()
            .order_by_desc(steps::Column::Id)
            .all(&self.conn)
            .await
    }

    /// 单个环节详情。
    pub async fn get_step(&self, id: i64) -> Result<Option<steps::Model>, sea_orm::DbErr> {
        steps::Entity::find_by_id(id).one(&self.conn).await
    }

    /// 创建环节（从 todo 复制数据）。
    pub async fn create_step(
        &self,
        title: &str,
        prompt: &str,
        executor: Option<&str>,
        acceptance_criteria: Option<&str>,
        source_todo_id: Option<i64>,
    ) -> Result<steps::Model, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = steps::ActiveModel {
            title: Set(title.to_string()),
            prompt: Set(prompt.to_string()),
            executor: Set(executor.map(|s| s.to_string())),
            acceptance_criteria: Set(acceptance_criteria.map(|s| s.to_string())),
            source_todo_id: Set(source_todo_id),
            created_at: Set(Some(now.clone())),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        am.insert(&self.conn).await
    }

    /// 统计某个 step 被多少 loop stage 引用。
    pub async fn count_loop_stages_using_step(
        &self,
        step_id: i64,
    ) -> Result<i64, sea_orm::DbErr> {
        use crate::db::entity::loop_stages;
        use sea_orm::EntityTrait;
        Ok(loop_stages::Entity::find()
            .filter(loop_stages::Column::TodoId.eq(step_id))
            .count(&self.conn)
            .await? as i64)
    }

    /// 批量统计多个 step 的引用计数。
    pub async fn count_loop_stages_for_steps(
        &self,
        step_ids: &[i64],
    ) -> Result<std::collections::HashMap<i64, i64>, sea_orm::DbErr> {
        use crate::db::entity::loop_stages;
        use sea_orm::{EntityTrait, QueryFilter};
        let mut map = std::collections::HashMap::new();
        for id in step_ids {
            let count = loop_stages::Entity::find()
                .filter(loop_stages::Column::TodoId.eq(*id))
                .count(&self.conn)
                .await? as i64;
            map.insert(*id, count);
        }
        Ok(map)
    }

    /// 列出环节 + 引用计数（供列表页使用）。
    pub async fn list_steps_with_usage_pure(&self) -> Result<Vec<(steps::Model, i64)>, sea_orm::DbErr> {
        let items = self.list_steps_pure().await?;
        let ids: Vec<i64> = items.iter().map(|s| s.id).collect();
        let usage = self.count_loop_stages_for_steps(&ids).await?;
        Ok(items
            .into_iter()
            .map(|s| {
                let count = usage.get(&s.id).copied().unwrap_or(0);
                (s, count)
            })
            .collect())
    }
}
