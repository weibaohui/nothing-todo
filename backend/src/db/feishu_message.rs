use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, QueryOrder};
use crate::db::Database;
use crate::db::entity::feishu_messages;

#[derive(Debug, Clone)]
pub struct FeishuMessageRecord {
    pub id: i64,
    pub bot_id: i64,
    pub message_id: String,
    pub chat_id: String,
    pub chat_type: String,
    pub sender_open_id: String,
    pub sender_nickname: Option<String>,
    pub sender_type: Option<String>,
    pub content: Option<String>,
    pub msg_type: String,
    pub is_mention: bool,
    pub processed: bool,
    pub is_history: bool,
    pub fetch_time: Option<String>,
    pub created_at: Option<String>,
}

impl Database {
    pub async fn save_feishu_message(
        &self,
        bot_id: i64,
        message_id: &str,
        chat_id: &str,
        chat_type: &str,
        sender_open_id: &str,
        content: Option<&str>,
        msg_type: &str,
        is_mention: bool,
    ) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = feishu_messages::ActiveModel {
            bot_id: ActiveValue::Set(bot_id),
            message_id: ActiveValue::Set(message_id.to_string()),
            chat_id: ActiveValue::Set(chat_id.to_string()),
            chat_type: ActiveValue::Set(chat_type.to_string()),
            sender_open_id: ActiveValue::Set(sender_open_id.to_string()),
            sender_nickname: ActiveValue::Set(None),
            content: ActiveValue::Set(content.map(String::from)),
            msg_type: ActiveValue::Set(msg_type.to_string()),
            is_mention: ActiveValue::Set(Some(is_mention)),
            processed: ActiveValue::Set(Some(false)),
            is_history: ActiveValue::Set(Some(false)),
            fetch_time: ActiveValue::Set(None),
            created_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await?;
        Ok(inserted.id)
    }

    pub async fn get_feishu_messages(
        &self,
        bot_id: i64,
        limit: u64,
    ) -> Result<Vec<FeishuMessageRecord>, sea_orm::DbErr> {
        let models = feishu_messages::Entity::find()
            .order_by_desc(feishu_messages::Column::Id)
            .all(&self.conn)
            .await?;

        Ok(models
            .into_iter()
            .filter(|m| m.bot_id == bot_id)
            .take(limit as usize)
            .map(|m| FeishuMessageRecord {
                id: m.id,
                bot_id: m.bot_id,
                message_id: m.message_id,
                chat_id: m.chat_id,
                chat_type: m.chat_type,
                sender_open_id: m.sender_open_id,
                sender_nickname: m.sender_nickname,
                sender_type: m.sender_type,
                content: m.content,
                msg_type: m.msg_type,
                is_mention: m.is_mention.unwrap_or(false),
                processed: m.processed.unwrap_or(false),
                is_history: m.is_history.unwrap_or(false),
                fetch_time: m.fetch_time,
                created_at: m.created_at,
            })
            .collect())
    }
}
