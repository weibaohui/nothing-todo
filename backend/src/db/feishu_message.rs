use std::collections::HashMap;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
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

    pub async fn save_feishu_history_message(
        &self,
        bot_id: i64,
        message_id: &str,
        chat_id: &str,
        chat_type: &str,
        sender_open_id: &str,
        sender_nickname: Option<&str>,
        sender_type: Option<&str>,
        content: Option<&str>,
        msg_type: &str,
        created_at: &str,
    ) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = feishu_messages::ActiveModel {
            bot_id: ActiveValue::Set(bot_id),
            message_id: ActiveValue::Set(message_id.to_string()),
            chat_id: ActiveValue::Set(chat_id.to_string()),
            chat_type: ActiveValue::Set(chat_type.to_string()),
            sender_open_id: ActiveValue::Set(sender_open_id.to_string()),
            sender_nickname: ActiveValue::Set(sender_nickname.map(String::from)),
            sender_type: ActiveValue::Set(sender_type.map(String::from)),
            content: ActiveValue::Set(content.map(String::from)),
            msg_type: ActiveValue::Set(msg_type.to_string()),
            is_mention: ActiveValue::Set(Some(false)),
            processed: ActiveValue::Set(Some(true)),
            is_history: ActiveValue::Set(Some(true)),
            fetch_time: ActiveValue::Set(Some(now)),
            created_at: ActiveValue::Set(Some(created_at.to_string())),
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

    pub async fn get_feishu_history_messages(
        &self,
        chat_id: Option<&str>,
        sender_open_id: Option<&str>,
        is_history: Option<bool>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<FeishuMessageRecord>, i64), sea_orm::DbErr> {
        let mut query = feishu_messages::Entity::find()
            .order_by_desc(feishu_messages::Column::CreatedAt);

        if let Some(sid) = sender_open_id {
            query = query.filter(feishu_messages::Column::SenderOpenId.eq(sid.to_string()));
        }

        if let Some(history) = is_history {
            query = query.filter(feishu_messages::Column::IsHistory.eq(Some(history)));
        }

        if let Some(cid) = chat_id {
            query = query.filter(feishu_messages::Column::ChatId.eq(cid.to_string()));
        }

        let total = query.clone().count(&self.conn).await? as i64;

        let offset = (page - 1) * page_size;
        let models = query
            .offset(offset)
            .limit(page_size)
            .all(&self.conn)
            .await?;

        let records = models
            .into_iter()
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
            .collect();

        Ok((records, total))
    }

    pub async fn feishu_message_exists(&self, message_id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = feishu_messages::Entity::find()
            .filter(feishu_messages::Column::MessageId.eq(message_id))
            .one(&self.conn)
            .await?;
        Ok(result.is_some())
    }

    pub async fn get_distinct_senders(&self) -> Result<Vec<(String, Option<String>, Option<String>, i64)>, sea_orm::DbErr> {
        // Returns distinct sender_open_ids with their message count, sender_type, and sender_nickname
        let models = feishu_messages::Entity::find()
            .order_by_desc(feishu_messages::Column::CreatedAt)
            .all(&self.conn)
            .await?;

        let mut sender_map: HashMap<String, (Option<String>, Option<String>, i64)> = HashMap::new();
        for model in models {
            let entry = sender_map.entry(model.sender_open_id.clone()).or_insert((
                model.sender_type.clone(),
                model.sender_nickname.clone(),
                0,
            ));
            entry.2 += 1;
        }

        let result: Vec<(String, Option<String>, Option<String>, i64)> = sender_map
            .into_iter()
            .map(|(sender_open_id, (sender_type, sender_nickname, count))| {
                (sender_open_id, sender_type, sender_nickname, count)
            })
            .collect();

        Ok(result)
    }

    /// Get the latest message create_time for a specific chat (for incremental fetching)
    pub async fn get_latest_history_message_time(
        &self,
        bot_id: i64,
        chat_id: &str,
    ) -> Result<Option<String>, sea_orm::DbErr> {
        let result = feishu_messages::Entity::find()
            .filter(feishu_messages::Column::BotId.eq(bot_id))
            .filter(feishu_messages::Column::ChatId.eq(chat_id.to_string()))
            .filter(feishu_messages::Column::IsHistory.eq(Some(true)))
            .order_by_desc(feishu_messages::Column::CreatedAt)
            .one(&self.conn)
            .await?;
        Ok(result.and_then(|m| m.created_at))
    }
}
