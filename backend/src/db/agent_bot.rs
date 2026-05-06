use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, QueryOrder};
use crate::db::Database;
use crate::db::entity::agent_bots;
use crate::models::AgentBot;

impl Database {
    pub async fn get_agent_bots(&self) -> Result<Vec<AgentBot>, sea_orm::DbErr> {
        let models = agent_bots::Entity::find()
            .order_by_desc(agent_bots::Column::Id)
            .all(&self.conn)
            .await?;
        Ok(models
            .into_iter()
            .map(|m| AgentBot {
                id: m.id,
                bot_type: m.bot_type,
                bot_name: m.bot_name,
                app_id: m.app_id,
                app_secret: m.app_secret,
                bot_open_id: m.bot_open_id,
                domain: m.domain,
                enabled: m.enabled.unwrap_or(true),
                created_at: m.created_at.unwrap_or_default(),
            })
            .collect())
    }

    pub async fn create_agent_bot(
        &self,
        bot_type: &str,
        bot_name: &str,
        app_id: &str,
        app_secret: &str,
        bot_open_id: Option<String>,
        domain: Option<String>,
    ) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = agent_bots::ActiveModel {
            bot_type: ActiveValue::Set(bot_type.to_string()),
            bot_name: ActiveValue::Set(bot_name.to_string()),
            app_id: ActiveValue::Set(app_id.to_string()),
            app_secret: ActiveValue::Set(app_secret.to_string()),
            bot_open_id: ActiveValue::Set(bot_open_id),
            domain: ActiveValue::Set(domain),
            enabled: ActiveValue::Set(Some(true)),
            created_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await?;
        Ok(inserted.id)
    }

    pub async fn delete_agent_bot(&self, id: i64) -> Result<(), sea_orm::DbErr> {
        agent_bots::Entity::delete_by_id(id).exec(&self.conn).await?;
        Ok(())
    }

    pub async fn get_agent_bot(&self, id: i64) -> Result<Option<AgentBot>, sea_orm::DbErr> {
        let model = agent_bots::Entity::find_by_id(id)
            .one(&self.conn)
            .await?;
        Ok(model.map(|m| AgentBot {
            id: m.id,
            bot_type: m.bot_type,
            bot_name: m.bot_name,
            app_id: m.app_id,
            app_secret: m.app_secret,
            bot_open_id: m.bot_open_id,
            domain: m.domain,
            enabled: m.enabled.unwrap_or(true),
            created_at: m.created_at.unwrap_or_default(),
        }))
    }
}
