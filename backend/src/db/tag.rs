use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};

use crate::db::Database;
use crate::db::entity::{tags, todo_tags, step_tags, loop_tags};
use crate::models::Tag;

use crate::models::TagBackup;

impl Database {
    pub async fn get_tags(&self) -> Result<Vec<Tag>, sea_orm::DbErr> {
        let models = tags::Entity::find()
            .order_by_asc(tags::Column::Name)
            .all(&self.conn)
            .await?;
        Ok(models
            .into_iter()
            .map(|m| Tag {
                id: m.id,
                name: m.name,
                color: m.color.unwrap_or_default(),
                created_at: m.created_at.unwrap_or_default(),
            })
            .collect())
    }

    pub async fn create_tag(&self, name: &str, color: &str) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = tags::ActiveModel {
            name: ActiveValue::Set(name.to_string()),
            color: ActiveValue::Set(Some(color.to_string())),
            created_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await?;
        Ok(inserted.id)
    }

    pub async fn delete_tag(&self, id: i64) -> Result<(), sea_orm::DbErr> {
        tags::Entity::delete_by_id(id).exec(&self.conn).await?;
        Ok(())
    }

    pub async fn add_todo_tag(&self, todo_id: i64, tag_id: i64) -> Result<(), sea_orm::DbErr> {
        let am = todo_tags::ActiveModel {
            todo_id: ActiveValue::Set(todo_id),
            tag_id: ActiveValue::Set(tag_id),
        };
        match todo_tags::Entity::insert(am)
            .on_conflict(
                sea_orm::sea_query::OnConflict::columns([
                    todo_tags::Column::TodoId,
                    todo_tags::Column::TagId,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec(&self.conn)
            .await
        {
            Ok(_) | Err(sea_orm::DbErr::RecordNotInserted) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn set_todo_tags(&self, todo_id: i64, tag_ids: &[i64]) -> Result<(), sea_orm::DbErr> {
        let tag_ids = tag_ids.to_vec();
        self
            .conn
            .transaction::<_, (), sea_orm::DbErr>(|txn| {
                Box::pin(async move {
                    todo_tags::Entity::delete_many()
                        .filter(todo_tags::Column::TodoId.eq(todo_id))
                        .exec(txn)
                        .await?;

                    if tag_ids.is_empty() {
                        return Ok(());
                    }

                    let rows: Vec<todo_tags::ActiveModel> = tag_ids
                        .into_iter()
                        .map(|tag_id| todo_tags::ActiveModel {
                            todo_id: ActiveValue::Set(todo_id),
                            tag_id: ActiveValue::Set(tag_id),
                        })
                        .collect();

                    todo_tags::Entity::insert_many(rows)
                        .on_conflict(
                            sea_orm::sea_query::OnConflict::columns([
                                todo_tags::Column::TodoId,
                                todo_tags::Column::TagId,
                            ])
                            .do_nothing()
                            .to_owned(),
                        )
                        .exec(txn)
                        .await?;

                    Ok(())
                })
            })
            .await
            .map_err(|e| match e {
                sea_orm::TransactionError::Connection(err) => err,
                sea_orm::TransactionError::Transaction(err) => err,
            })?;
        Ok(())
    }

    pub async fn get_tag_backups(&self) -> Result<Vec<TagBackup>, sea_orm::DbErr> {
        Ok(tags::Entity::find()
            .all(&self.conn)
            .await?
            .into_iter()
            .map(|m| TagBackup {
                name: m.name,
                color: m.color.unwrap_or_default(),
            })
            .collect())
    }

    /// 查询指定 todo 当前关联的所有 tag_id。
    pub async fn get_todo_tag_ids(&self, todo_id: i64) -> Result<Vec<i64>, sea_orm::DbErr> {
        use sea_orm::ColumnTrait;
        let rows = todo_tags::Entity::find()
            .filter(todo_tags::Column::TodoId.eq(todo_id))
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(|r| r.tag_id).collect())
    }

    pub async fn find_tag_by_name(&self, name: &str) -> Result<Option<i64>, sea_orm::DbErr> {
        use sea_orm::ColumnTrait;
        Ok(tags::Entity::find()
            .filter(tags::Column::Name.eq(name))
            .one(&self.conn)
            .await?
            .map(|m| m.id))
    }

    // ====== Step Tags ======

    /// 设置环节的标签（全量替换）。
    pub async fn set_step_tags(&self, step_id: i64, tag_ids: &[i64]) -> Result<(), sea_orm::DbErr> {
        let tag_ids = tag_ids.to_vec();
        self.conn
            .transaction::<_, (), sea_orm::DbErr>(|txn| {
                Box::pin(async move {
                    step_tags::Entity::delete_many()
                        .filter(step_tags::Column::StepId.eq(step_id))
                        .exec(txn)
                        .await?;

                    if tag_ids.is_empty() {
                        return Ok(());
                    }

                    let rows: Vec<step_tags::ActiveModel> = tag_ids
                        .into_iter()
                        .map(|tag_id| step_tags::ActiveModel {
                            step_id: ActiveValue::Set(step_id),
                            tag_id: ActiveValue::Set(tag_id),
                        })
                        .collect();

                    step_tags::Entity::insert_many(rows)
                        .on_conflict(
                            sea_orm::sea_query::OnConflict::columns([
                                step_tags::Column::StepId,
                                step_tags::Column::TagId,
                            ])
                            .do_nothing()
                            .to_owned(),
                        )
                        .exec(txn)
                        .await?;

                    Ok(())
                })
            })
            .await
            .map_err(|e| match e {
                sea_orm::TransactionError::Connection(err) => err,
                sea_orm::TransactionError::Transaction(err) => err,
            })?;
        Ok(())
    }

    /// 查询环节关联的所有标签 ID。
    pub async fn get_step_tag_ids(&self, step_id: i64) -> Result<Vec<i64>, sea_orm::DbErr> {
        use sea_orm::ColumnTrait;
        let rows = step_tags::Entity::find()
            .filter(step_tags::Column::StepId.eq(step_id))
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(|r| r.tag_id).collect())
    }

    // ====== Loop Tags ======

    /// 设置环路的标签（全量替换）。
    pub async fn set_loop_tags(&self, loop_id: i64, tag_ids: &[i64]) -> Result<(), sea_orm::DbErr> {
        let tag_ids = tag_ids.to_vec();
        self.conn
            .transaction::<_, (), sea_orm::DbErr>(|txn| {
                Box::pin(async move {
                    loop_tags::Entity::delete_many()
                        .filter(loop_tags::Column::LoopId.eq(loop_id))
                        .exec(txn)
                        .await?;

                    if tag_ids.is_empty() {
                        return Ok(());
                    }

                    let rows: Vec<loop_tags::ActiveModel> = tag_ids
                        .into_iter()
                        .map(|tag_id| loop_tags::ActiveModel {
                            loop_id: ActiveValue::Set(loop_id),
                            tag_id: ActiveValue::Set(tag_id),
                        })
                        .collect();

                    loop_tags::Entity::insert_many(rows)
                        .on_conflict(
                            sea_orm::sea_query::OnConflict::columns([
                                loop_tags::Column::LoopId,
                                loop_tags::Column::TagId,
                            ])
                            .do_nothing()
                            .to_owned(),
                        )
                        .exec(txn)
                        .await?;

                    Ok(())
                })
            })
            .await
            .map_err(|e| match e {
                sea_orm::TransactionError::Connection(err) => err,
                sea_orm::TransactionError::Transaction(err) => err,
            })?;
        Ok(())
    }

    /// 查询环路关联的所有标签 ID。
    pub async fn get_loop_tag_ids(&self, loop_id: i64) -> Result<Vec<i64>, sea_orm::DbErr> {
        use sea_orm::ColumnTrait;
        let rows = loop_tags::Entity::find()
            .filter(loop_tags::Column::LoopId.eq(loop_id))
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(|r| r.tag_id).collect())
    }
}
