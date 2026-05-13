use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

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
        session_dir: m.session_dir,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

struct DefaultExecutor {
    name: &'static str,
    path: &'static str,
    display_name: &'static str,
    session_dir: &'static str,
}

const DEFAULT_EXECUTORS: &[DefaultExecutor] = &[
    DefaultExecutor { name: "claudecode", path: "claude", display_name: "Claude Code", session_dir: "~/.claude" },
    DefaultExecutor { name: "joinai", path: "joinai", display_name: "JoinAI", session_dir: "" },
    DefaultExecutor { name: "codebuddy", path: "codebuddy", display_name: "CodeBuddy", session_dir: "~/.codebuddy" },
    DefaultExecutor { name: "opencode", path: "opencode", display_name: "Opencode", session_dir: "~/.opencode" },
    DefaultExecutor { name: "atomcode", path: "atomcode", display_name: "AtomCode", session_dir: "~/.atomcode" },
    DefaultExecutor { name: "hermes", path: "hermes", display_name: "Hermes", session_dir: "~/.hermes" },
    DefaultExecutor { name: "kimi", path: "kimi", display_name: "Kimi", session_dir: "~/.kimi" },
    DefaultExecutor { name: "codex", path: "codex", display_name: "Codex", session_dir: "~/.codex" },
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
        session_dir: Option<&str>,
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
            if let Some(sd) = session_dir {
                am.session_dir = ActiveValue::Set(sd.to_string());
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
        let count = executors::Entity::find().count(&self.conn).await?;
        if count > 0 {
            return Ok(());
        }

        let now = crate::models::utc_timestamp();

        for d in DEFAULT_EXECUTORS {
            let path = match d.name {
                "claudecode" => &cfg_executors.claude_code,
                "joinai" => &cfg_executors.joinai,
                "codebuddy" => &cfg_executors.codebuddy,
                "opencode" => &cfg_executors.opencode,
                "atomcode" => &cfg_executors.atomcode,
                "hermes" => &cfg_executors.hermes,
                "kimi" => &cfg_executors.kimi,
                "codex" => &cfg_executors.codex,
                _ => continue,
            };
            let am = executors::ActiveModel {
                name: ActiveValue::Set(d.name.to_string()),
                path: ActiveValue::Set(path.to_string()),
                enabled: ActiveValue::Set(true),
                display_name: ActiveValue::Set(d.display_name.to_string()),
                session_dir: ActiveValue::Set(d.session_dir.to_string()),
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
        let count = executors::Entity::find().count(&self.conn).await?;
        if count > 0 {
            return Ok(());
        }

        let now = crate::models::utc_timestamp();
        for d in DEFAULT_EXECUTORS {
            let am = executors::ActiveModel {
                name: ActiveValue::Set(d.name.to_string()),
                path: ActiveValue::Set(d.path.to_string()),
                enabled: ActiveValue::Set(true),
                display_name: ActiveValue::Set(d.display_name.to_string()),
                session_dir: ActiveValue::Set(d.session_dir.to_string()),
                created_at: ActiveValue::Set(Some(now.clone())),
                updated_at: ActiveValue::Set(Some(now.clone())),
                ..Default::default()
            };
            am.insert(&self.conn).await?;
        }

        tracing::info!("Seeded default executors into database");
        Ok(())
    }

    /// Backfill session_dir for existing executors that have empty session_dir.
    pub async fn backfill_session_dir(&self) -> Result<(), sea_orm::DbErr> {
        let models = executors::Entity::find().all(&self.conn).await?;
        for m in models {
            if m.session_dir.is_empty() {
                let default_dir = DEFAULT_EXECUTORS.iter()
                    .find(|d| d.name == m.name)
                    .map(|d| d.session_dir)
                    .unwrap_or("");
                if !default_dir.is_empty() {
                    let mut am: executors::ActiveModel = m.into();
                    am.session_dir = ActiveValue::Set(default_dir.to_string());
                    am.update(&self.conn).await?;
                }
            }
        }
        Ok(())
    }
}
