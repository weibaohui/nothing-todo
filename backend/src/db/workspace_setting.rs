//! 工作空间设置的数据库访问层
//!
//! 提供 workspace_settings 表的 CRUD 操作。

use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use crate::db::Database;

/// 获取工作空间设置
pub async fn get_workspace_settings(
    db: &Database,
    workspace_id: i64,
) -> Result<Option<crate::db::entity::workspace_settings::Model>, sea_orm::DbErr> {
    use crate::db::entity::workspace_settings as ws;

    let settings = ws::Entity::find()
        .filter(ws::Column::WorkspaceId.eq(workspace_id))
        .one(&db.conn)
        .await?;

    Ok(settings)
}

/// 创建或更新工作空间设置
pub async fn upsert_workspace_settings(
    db: &Database,
    workspace_id: i64,
    default_response_todo_id: Option<i64>,
) -> Result<(), sea_orm::DbErr> {
    use crate::db::entity::workspace_settings as ws;

    let existing = ws::Entity::find()
        .filter(ws::Column::WorkspaceId.eq(workspace_id))
        .one(&db.conn)
        .await?;

    if let Some(model) = existing {
        // 更新
        let mut am = model.into_active_model();
        if let Some(todo_id) = default_response_todo_id {
            am.default_response_todo_id = ActiveValue::Set(Some(todo_id));
        }
        am.updated_at = ActiveValue::Set(Some(crate::models::utc_timestamp()));
        am.update(&db.conn).await?;
    } else {
        // 创建
        let now = crate::models::utc_timestamp();
        let am = ws::ActiveModel {
            workspace_id: ActiveValue::Set(workspace_id),
            default_response_todo_id: ActiveValue::Set(default_response_todo_id),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        am.insert(&db.conn).await?;
    }

    Ok(())
}
