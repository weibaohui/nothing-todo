use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::db::entity::project_directories;
use crate::db::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDirectory {
    pub id: i64,
    pub path: String,
    pub name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Database {
    pub async fn get_project_directories(&self) -> Result<Vec<ProjectDirectory>, sea_orm::DbErr> {
        let models = project_directories::Entity::find()
            .order_by_asc(project_directories::Column::Path)
            .all(&self.conn)
            .await?;

        Ok(models
            .into_iter()
            .map(|m| ProjectDirectory {
                id: m.id,
                path: m.path,
                name: m.name,
                created_at: m.created_at,
                updated_at: m.updated_at,
            })
            .collect())
    }

    pub async fn create_project_directory(
        &self,
        path: &str,
        name: Option<&str>,
    ) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = project_directories::ActiveModel {
            path: ActiveValue::Set(path.to_string()),
            name: ActiveValue::Set(name.map(|s| s.to_string())),
            created_at: ActiveValue::Set(Some(now.clone())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await?;
        Ok(inserted.id)
    }

    pub async fn update_project_directory(
        &self,
        id: i64,
        name: Option<&str>,
    ) -> Result<(), sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = project_directories::ActiveModel {
            id: ActiveValue::Unchanged(id),
            name: ActiveValue::Set(name.map(|s| s.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        self.exec_update(am).await
    }

    pub async fn delete_project_directory(&self, id: i64) -> Result<(), sea_orm::DbErr> {
        project_directories::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await
            .map(|_| ())
    }

    pub async fn get_project_directory_by_path(
        &self,
        path: &str,
    ) -> Result<Option<ProjectDirectory>, sea_orm::DbErr> {
        let model = project_directories::Entity::find()
            .filter(project_directories::Column::Path.eq(path))
            .one(&self.conn)
            .await?;

        Ok(model.map(|m| ProjectDirectory {
            id: m.id,
            path: m.path,
            name: m.name,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }))
    }

    pub async fn get_project_directory_by_id(
        &self,
        id: i64,
    ) -> Result<Option<ProjectDirectory>, sea_orm::DbErr> {
        let model = project_directories::Entity::find_by_id(id)
            .one(&self.conn)
            .await?;

        Ok(model.map(|m| ProjectDirectory {
            id: m.id,
            path: m.path,
            name: m.name,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }))
    }

    /// 如果目录不存在则创建，返回目录信息
    /// 处理并发竞态：捕获唯一约束冲突并重试查询
    pub async fn get_or_create_project_directory(
        &self,
        path: &str,
    ) -> Result<ProjectDirectory, sea_orm::DbErr> {
        if let Some(existing) = self.get_project_directory_by_path(path).await? {
            return Ok(existing);
        }

        match self.create_project_directory(path, None).await {
            Ok(id) => {
                // 创建成功后从数据库查询以获取准确的时间戳
                self.get_project_directory_by_id(id)
                    .await?
                    .ok_or_else(|| sea_orm::DbErr::Custom("Failed to retrieve created directory".into()))
            }
            Err(e) => {
                // 如果是唯一约束冲突，说明另一个请求已经创建了该目录，重试查询
                if is_unique_constraint_error(&e) {
                    self.get_project_directory_by_path(path)
                        .await?
                        .ok_or_else(|| sea_orm::DbErr::Custom("Directory disappeared after conflict".into()))
                } else {
                    Err(e)
                }
            }
        }
    }
}

fn is_unique_constraint_error(err: &sea_orm::DbErr) -> bool {
    let err_str = format!("{:?}", err);
    err_str.contains("UNIQUE constraint failed")
}