use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::db::entity::executors;
use crate::db::Database;
use crate::models::ExecutorConfig;

fn map_executor(m: executors::Model) -> ExecutorConfig {
    ExecutorConfig {
        id: m.id,
        name: m.name,
        path: m.path,
        enabled: m.enabled,
        display_name: m.display_name,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

struct DefaultExecutor {
    name: &'static str,
    path: &'static str,
    display_name: &'static str,
}

const DEFAULT_EXECUTORS: &[DefaultExecutor] = &[
    DefaultExecutor { name: "claude_code", path: "claude", display_name: "Claude Code" },
    DefaultExecutor { name: "joinai", path: "joinai", display_name: "JoinAI" },
    DefaultExecutor { name: "codebuddy", path: "codebuddy", display_name: "CodeBuddy" },
    DefaultExecutor { name: "opencode", path: "opencode", display_name: "Opencode" },
    DefaultExecutor { name: "atomcode", path: "atomcode", display_name: "AtomCode" },
    DefaultExecutor { name: "hermes", path: "hermes", display_name: "Hermes" },
    DefaultExecutor { name: "kimi", path: "kimi", display_name: "Kimi" },
    DefaultExecutor { name: "codex", path: "codex", display_name: "Codex" },
];

impl Database {
    pub async fn get_executors(&self) -> Result<Vec<ExecutorConfig>, sea_orm::DbErr> {
        let models = executors::Entity::find()
            .order_by_asc(executors::Column::Id)
            .all(&self.conn)
            .await?;
        Ok(models.into_iter().map(map_executor).collect())
    }

    pub async fn get_enabled_executors(&self) -> Result<Vec<ExecutorConfig>, sea_orm::DbErr> {
        let models = executors::Entity::find()
            .filter(executors::Column::Enabled.eq(true))
            .order_by_asc(executors::Column::Id)
            .all(&self.conn)
            .await?;
        Ok(models.into_iter().map(map_executor).collect())
    }

    pub async fn get_executor_by_name(&self, name: &str) -> Result<Option<ExecutorConfig>, sea_orm::DbErr> {
        let model = executors::Entity::find()
            .filter(executors::Column::Name.eq(name))
            .one(&self.conn)
            .await?;
        Ok(model.map(map_executor))
    }

    pub async fn update_executor(
        &self,
        name: &str,
        path: Option<&str>,
        enabled: Option<bool>,
        display_name: Option<&str>,
    ) -> Result<(), sea_orm::DbErr> {
        let model = executors::Entity::find()
            .filter(executors::Column::Name.eq(name))
            .one(&self.conn)
            .await?;
        if let Some(m) = model {
            let now = crate::models::utc_timestamp();
            let mut am: executors::ActiveModel = m.into();
            if let Some(p) = path {
                am.path = ActiveValue::Set(p.to_string());
            }
            if let Some(e) = enabled {
                am.enabled = ActiveValue::Set(e);
            }
            if let Some(d) = display_name {
                am.display_name = ActiveValue::Set(d.to_string());
            }
            am.updated_at = ActiveValue::Set(Some(now));
            am.update(&self.conn).await?;
        }
        Ok(())
    }

    /// Migrate executor paths from config.yaml into database.
    /// Only runs when the executors table is empty.
    pub async fn migrate_from_config(
        &self,
        cfg_executors: &crate::config::ExecutorPaths,
    ) -> Result<(), sea_orm::DbErr> {
        let count = executors::Entity::find().all(&self.conn).await?;
        if !count.is_empty() {
            return Ok(());
        }

        let now = crate::models::utc_timestamp();
        let pairs: [(&str, &str); 8] = [
            ("claude_code", &cfg_executors.claude_code),
            ("joinai", &cfg_executors.joinai),
            ("codebuddy", &cfg_executors.codebuddy),
            ("opencode", &cfg_executors.opencode),
            ("atomcode", &cfg_executors.atomcode),
            ("hermes", &cfg_executors.hermes),
            ("kimi", &cfg_executors.kimi),
            ("codex", &cfg_executors.codex),
        ];

        for (name, path) in &pairs {
            let default = DEFAULT_EXECUTORS.iter().find(|d| d.name == *name).unwrap();
            let am = executors::ActiveModel {
                name: ActiveValue::Set(name.to_string()),
                path: ActiveValue::Set(path.to_string()),
                enabled: ActiveValue::Set(true),
                display_name: ActiveValue::Set(default.display_name.to_string()),
                created_at: ActiveValue::Set(Some(now.clone())),
                updated_at: ActiveValue::Set(Some(now.clone())),
                ..Default::default()
            };
            am.insert(&self.conn).await?;
        }

        tracing::info!("Migrated executor paths from config.yaml to database");
        Ok(())
    }

    /// Seed default executors if table is empty (fresh install).
    pub async fn seed_default_executors(&self) -> Result<(), sea_orm::DbErr> {
        let count = executors::Entity::find().all(&self.conn).await?;
        if !count.is_empty() {
            return Ok(());
        }

        let now = crate::models::utc_timestamp();
        for d in DEFAULT_EXECUTORS {
            let am = executors::ActiveModel {
                name: ActiveValue::Set(d.name.to_string()),
                path: ActiveValue::Set(d.path.to_string()),
                enabled: ActiveValue::Set(true),
                display_name: ActiveValue::Set(d.display_name.to_string()),
                created_at: ActiveValue::Set(Some(now.clone())),
                updated_at: ActiveValue::Set(Some(now.clone())),
                ..Default::default()
            };
            am.insert(&self.conn).await?;
        }

        tracing::info!("Seeded default executors into database");
        Ok(())
    }
}
