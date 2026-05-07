use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};
use crate::db::Database;
use crate::db::entity::feishu_push_targets;

impl Database {
    /// Set or update the push target for a bot.
    /// push_level: "disabled", "result_only", or "all". Defaults to "result_only" for new targets.
    pub async fn set_feishu_push_target(
        &self,
        bot_id: i64,
        chat_id: Option<&str>,
        receive_id: &str,
        receive_id_type: &str,
        push_level: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();

        let existing = feishu_push_targets::Entity::find()
            .all(&self.conn)
            .await?
            .into_iter()
            .find(|t| t.bot_id == bot_id);

        if let Some(t) = existing {
            let mut am: feishu_push_targets::ActiveModel = t.into();
            am.chat_id = ActiveValue::Set(chat_id.map(String::from));
            am.receive_id = ActiveValue::Set(receive_id.to_string());
            am.receive_id_type = ActiveValue::Set(receive_id_type.to_string());
            am.push_level = ActiveValue::Set(push_level.to_string());
            am.updated_at = ActiveValue::Set(Some(now));
            am.update(&self.conn).await?;
        } else {
            let am = feishu_push_targets::ActiveModel {
                bot_id: ActiveValue::Set(bot_id),
                chat_id: ActiveValue::Set(chat_id.map(String::from)),
                receive_id: ActiveValue::Set(receive_id.to_string()),
                receive_id_type: ActiveValue::Set(receive_id_type.to_string()),
                push_level: ActiveValue::Set(push_level.to_string()),
                created_at: ActiveValue::Set(Some(now.clone())),
                updated_at: ActiveValue::Set(Some(now)),
                ..Default::default()
            };
            am.insert(&self.conn).await?;
        }

        Ok(())
    }

    /// Get the push target for a bot.
    pub async fn get_feishu_push_target(
        &self,
        bot_id: i64,
    ) -> Result<Option<feishu_push_targets::Model>, sea_orm::DbErr> {
        let targets = feishu_push_targets::Entity::find()
            .all(&self.conn)
            .await?;
        Ok(targets.into_iter().find(|t| t.bot_id == bot_id))
    }

    /// Get all bots with push enabled (push_level != "disabled").
    pub async fn get_enabled_feishu_push_targets(
        &self,
    ) -> Result<Vec<feishu_push_targets::Model>, sea_orm::DbErr> {
        let targets = feishu_push_targets::Entity::find()
            .all(&self.conn)
            .await?;
        Ok(targets
            .into_iter()
            .filter(|t| t.push_level != "disabled")
            .collect())
    }

    /// Update push level for a bot.
    pub async fn update_feishu_push_level(
        &self,
        bot_id: i64,
        push_level: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();

        let existing = feishu_push_targets::Entity::find()
            .all(&self.conn)
            .await?
            .into_iter()
            .find(|t| t.bot_id == bot_id);

        match existing {
            Some(t) => {
                let mut am: feishu_push_targets::ActiveModel = t.into();
                am.push_level = ActiveValue::Set(push_level.to_string());
                am.updated_at = ActiveValue::Set(Some(now));
                am.update(&self.conn).await?;
            }
            None => {}
        }
        Ok(())
    }
}
