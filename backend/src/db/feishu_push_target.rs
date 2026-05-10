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
            am.updated_at = ActiveValue::Set(Some(now.clone()));
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
                updated_at: ActiveValue::Set(Some(now.clone())),
                ..Default::default()
            };

            // Try insert; if a UNIQUE constraint error happens (race), attempt to update the existing row.
            match am.insert(&self.conn).await {
                Ok(_) => {}
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("UNIQUE constraint failed") {
                        // Possible race or old DB schema: try to find and update an existing row.
                        // Prefer matching (bot_id, target_type); fall back to matching bot_id only.
                        let found = match feishu_push_targets::Entity::find()
                            .filter(feishu_push_targets::Column::BotId.eq(bot_id))
                            .filter(feishu_push_targets::Column::TargetType.eq(target_type))
                            .one(&self.conn)
                            .await
                        {
                            Ok(Some(m)) => Some(m),
                            Ok(None) => match feishu_push_targets::Entity::find()
                                .filter(feishu_push_targets::Column::BotId.eq(bot_id))
                                .one(&self.conn)
                                .await
                            {
                                Ok(Some(m2)) => Some(m2),
                                Ok(None) => None,
                                Err(_) => return Err(e),
                            },
                            Err(_) => return Err(e),
                        };

                        if let Some(existing2) = found {
                            let mut am2: feishu_push_targets::ActiveModel = existing2.into();
                            // Ensure target_type is set to requested value (safe even if schema has target_type column).
                            am2.target_type = ActiveValue::Set(target_type.to_string());
                            am2.chat_id = ActiveValue::Set(chat_id.map(String::from));
                            am2.receive_id = ActiveValue::Set(receive_id.to_string());
                            am2.receive_id_type = ActiveValue::Set(receive_id_type.to_string());
                            am2.push_level = ActiveValue::Set(push_level.to_string());
                            am2.updated_at = ActiveValue::Set(Some(now.clone()));
                            am2.update(&self.conn).await?;
                        } else {
                            return Err(e);
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
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
}
