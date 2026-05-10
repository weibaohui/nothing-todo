use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use crate::db::Database;
use crate::db::entity::feishu_push_targets;

impl Database {
    /// Set or update the push target for a bot.
    /// target_type: "p2p" or "group"
    /// push_level: "disabled", "result_only", or "all". Defaults to "result_only" for new targets.
    pub async fn set_feishu_push_target(
        &self,
        bot_id: i64,
        target_type: &str,
        chat_id: Option<&str>,
        receive_id: &str,
        receive_id_type: &str,
        push_level: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();

        // Find existing target for this bot and target_type
        let existing = feishu_push_targets::Entity::find()
            .filter(feishu_push_targets::Column::BotId.eq(bot_id))
            .filter(feishu_push_targets::Column::TargetType.eq(target_type))
            .one(&self.conn)
            .await?;

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
                target_type: ActiveValue::Set(target_type.to_string()),
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

    /// Get the push target for a bot and target type.
    pub async fn get_feishu_push_target(
        &self,
        bot_id: i64,
        target_type: &str,
    ) -> Result<Option<feishu_push_targets::Model>, sea_orm::DbErr> {
        let target = feishu_push_targets::Entity::find()
            .filter(feishu_push_targets::Column::BotId.eq(bot_id))
            .filter(feishu_push_targets::Column::TargetType.eq(target_type))
            .one(&self.conn)
            .await?;
        Ok(target)
    }

    /// Get all bots with push enabled (push_level != "disabled") for a specific target type.
    pub async fn get_enabled_feishu_push_targets(
        &self,
        target_type: &str,
    ) -> Result<Vec<feishu_push_targets::Model>, sea_orm::DbErr> {
        let targets = feishu_push_targets::Entity::find()
            .filter(feishu_push_targets::Column::TargetType.eq(target_type))
            .all(&self.conn)
            .await?;
        Ok(targets
            .into_iter()
            .filter(|t| t.push_level != "disabled")
            .collect())
    }

    /// Get all bots with push enabled (push_level != "disabled"), regardless of target type.
    pub async fn get_all_enabled_feishu_push_targets(
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

    /// Get all push targets configured for group chat (target_type = "group").
    /// Returns (bot_id, chat_id) pairs.
    pub async fn get_group_chat_push_targets(
        &self,
    ) -> Result<Vec<(i64, String)>, sea_orm::DbErr> {
        let targets = feishu_push_targets::Entity::find()
            .filter(feishu_push_targets::Column::TargetType.eq("group"))
            .all(&self.conn)
            .await?;
        Ok(targets
            .into_iter()
            .filter(|t| !t.receive_id.is_empty())
            .map(|t| (t.bot_id, t.receive_id))
            .collect())
    }

    /// Update push level for a bot and target type.
    pub async fn update_feishu_push_level(
        &self,
        bot_id: i64,
        target_type: &str,
        push_level: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();

        let existing = feishu_push_targets::Entity::find()
            .filter(feishu_push_targets::Column::BotId.eq(bot_id))
            .filter(feishu_push_targets::Column::TargetType.eq(target_type))
            .one(&self.conn)
            .await?;

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

    /// Get response enabled status for a specific chat type.
    /// Returns (p2p_response_enabled, group_response_enabled).
    pub async fn get_feishu_response_enabled(
        &self,
        bot_id: i64,
    ) -> Result<(bool, bool), sea_orm::DbErr> {
        let targets = feishu_push_targets::Entity::find()
            .filter(feishu_push_targets::Column::BotId.eq(bot_id))
            .all(&self.conn)
            .await?;

        let mut p2p_enabled = false;
        let mut group_enabled = false;

        for t in targets {
            if t.target_type == "p2p" {
                p2p_enabled = t.p2p_response_enabled;
            } else if t.target_type == "group" {
                group_enabled = t.group_response_enabled;
            }
        }

        Ok((p2p_enabled, group_enabled))
    }

    /// Update response enabled status for a specific target type.
    pub async fn update_feishu_response_enabled(
        &self,
        bot_id: i64,
        target_type: &str,
        enabled: bool,
    ) -> Result<(), sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();

        let existing = feishu_push_targets::Entity::find()
            .filter(feishu_push_targets::Column::BotId.eq(bot_id))
            .filter(feishu_push_targets::Column::TargetType.eq(target_type))
            .one(&self.conn)
            .await?;

        match existing {
            Some(t) => {
                let mut am: feishu_push_targets::ActiveModel = t.into();
                if target_type == "p2p" {
                    am.p2p_response_enabled = ActiveValue::Set(enabled);
                } else {
                    am.group_response_enabled = ActiveValue::Set(enabled);
                }
                am.updated_at = ActiveValue::Set(Some(now));
                am.update(&self.conn).await?;
            }
            None => {}
        }
        Ok(())
    }
}
