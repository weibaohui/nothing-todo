use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};
use crate::db::Database;
use crate::db::entity::feishu_homes;

impl Database {
    pub async fn set_feishu_home(
        &self,
        bot_id: i64,
        user_open_id: &str,
        chat_id: Option<&str>,
        receive_id: &str,
        receive_id_type: &str,
    ) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();

        // Try to find existing
        let existing = feishu_homes::Entity::find()
            .all(&self.conn)
            .await?
            .into_iter()
            .find(|h| h.bot_id == bot_id && h.user_open_id == user_open_id);

        if let Some(h) = existing {
            let mut am: feishu_homes::ActiveModel = h.into();
            am.chat_id = ActiveValue::Set(chat_id.map(String::from));
            am.receive_id = ActiveValue::Set(receive_id.to_string());
            am.receive_id_type = ActiveValue::Set(receive_id_type.to_string());
            am.updated_at = ActiveValue::Set(Some(now));
            let updated = am.update(&self.conn).await?;
            Ok(updated.id)
        } else {
            let am = feishu_homes::ActiveModel {
                bot_id: ActiveValue::Set(bot_id),
                user_open_id: ActiveValue::Set(user_open_id.to_string()),
                chat_id: ActiveValue::Set(chat_id.map(String::from)),
                receive_id: ActiveValue::Set(receive_id.to_string()),
                receive_id_type: ActiveValue::Set(receive_id_type.to_string()),
                created_at: ActiveValue::Set(Some(now)),
                ..Default::default()
            };
            let inserted = am.insert(&self.conn).await?;
            Ok(inserted.id)
        }
    }

    pub async fn get_feishu_home(
        &self,
        bot_id: i64,
        user_open_id: &str,
    ) -> Result<Option<(String, String)>, sea_orm::DbErr> {
        let homes = feishu_homes::Entity::find()
            .all(&self.conn)
            .await?;
        Ok(homes
            .into_iter()
            .find(|h| h.bot_id == bot_id && h.user_open_id == user_open_id)
            .map(|h| (h.receive_id, h.receive_id_type)))
    }
}
