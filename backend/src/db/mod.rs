//! Database access layer (SeaORM).
//!
//! - Fixed database path: `~/.ntd/data.db`
//! - Built-in SQLite (libsqlite3-sys/bundled), no system dependencies
//! - All public methods are async

use std::str::FromStr;
use std::time::Duration;

use chrono::Utc;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ConnectOptions, ConnectionTrait, Database as SeaDatabase,
    DatabaseConnection, DbBackend, EntityTrait, IntoActiveModel, Statement,
};

pub mod entity;
pub use entity::prelude::*;

fn compute_next_run(cron_expr: &str) -> Option<String> {
    cron::Schedule::from_str(cron_expr).ok().and_then(|schedule| {
        schedule
            .upcoming(Utc)
            .next()
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
    })
}

pub struct Database {
    pub(super) conn: DatabaseConnection,
}

impl Database {
    /// Open database connection (async).
    /// path: database file path or ":memory:".
    pub async fn new(path: &str) -> Result<Self, sea_orm::DbErr> {
        let url = if path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{}?mode=rwc", path)
        };

        let mut opt = ConnectOptions::new(url);
        opt.max_connections(8)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(5))
            .sqlx_logging(false);

        let conn = SeaDatabase::connect(opt).await?;
        let db = Self { conn };

        // Set busy_timeout via PRAGMA (SQLite connection-level setting)
        db.exec("PRAGMA busy_timeout = 5000").await?;

        db.init_tables().await?;
        Ok(db)
    }

    pub(super) async fn exec(&self, sql: &str) -> Result<(), sea_orm::DbErr> {
        self.conn
            .execute(Statement::from_string(DbBackend::Sqlite, sql.to_string()))
            .await
            .map(|_| ())
    }

    pub(super) async fn exec_update<M>(&self, model: M) -> Result<(), sea_orm::DbErr>
    where
        M: ActiveModelTrait + ActiveModelBehavior + Send,
        <<M as ActiveModelTrait>::Entity as EntityTrait>::Model: IntoActiveModel<M>,
    {
        model.update(&self.conn).await.map(|_| ())
    }

    async fn init_tables(&self) -> Result<(), sea_orm::DbErr> {
        self.exec(
            "CREATE TABLE IF NOT EXISTS todos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                prompt TEXT DEFAULT '',
                status TEXT DEFAULT 'pending',
                created_at TEXT,
                updated_at TEXT,
                deleted_at TEXT,
                executor TEXT DEFAULT 'claudecode',
                scheduler_enabled INTEGER DEFAULT 0,
                scheduler_config TEXT,
                task_id TEXT,
                workspace TEXT
            )",
        )
        .await?;

        self.exec(
            "CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT DEFAULT '#1890ff',
                created_at TEXT
            )",
        )
        .await?;

        self.exec(
            "CREATE TABLE IF NOT EXISTS todo_tags (
                todo_id INTEGER,
                tag_id INTEGER,
                PRIMARY KEY (todo_id, tag_id),
                FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
        )
        .await?;

        self.exec(
            "CREATE TABLE IF NOT EXISTS execution_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                todo_id INTEGER,
                status TEXT DEFAULT 'running',
                command TEXT,
                stdout TEXT DEFAULT '',
                stderr TEXT DEFAULT '',
                logs TEXT DEFAULT '[]',
                result TEXT,
                usage TEXT,
                executor TEXT,
                model TEXT,
                started_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                finished_at TEXT,
                trigger_type TEXT DEFAULT 'manual',
                pid INTEGER,
                task_id TEXT,
                session_id TEXT,
                FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
            )",
        )
        .await?;

        // 添加 pid 字段的迁移（向后兼容）
        self.exec(
            "ALTER TABLE execution_records ADD COLUMN pid INTEGER"
        )
        .await.ok(); // 忽略错误，因为字段可能已存在

        // 添加 task_id 字段的迁移（向后兼容）
        self.exec(
            "ALTER TABLE execution_records ADD COLUMN task_id TEXT"
        )
        .await.ok(); // 忽略错误，因为字段可能已存在

        // 添加 session_id 字段的迁移（向后兼容）
        self.exec(
            "ALTER TABLE execution_records ADD COLUMN session_id TEXT"
        )
        .await.ok(); // 忽略错误，因为字段可能已存在

        // 添加 workspace 字段的迁移（向后兼容）
        self.exec(
            "ALTER TABLE todos ADD COLUMN workspace TEXT"
        )
        .await.ok(); // 忽略错误，因为字段可能已存在

        // 添加 todo_progress 字段的迁移（向后兼容）
        self.exec(
            "ALTER TABLE execution_records ADD COLUMN todo_progress TEXT"
        )
        .await.ok(); // 忽略错误，因为字段可能已存在

        // 添加 execution_stats 字段的迁移（向后兼容）
        self.exec(
            "ALTER TABLE execution_records ADD COLUMN execution_stats TEXT"
        )
        .await.ok(); // 忽略错误，因为字段可能已存在

        // 添加 resume_message 字段的迁移（向后兼容）
        self.exec(
            "ALTER TABLE execution_records ADD COLUMN resume_message TEXT"
        )
        .await.ok(); // 忽略错误，因为字段可能已存在

        // Skill invocations tracking table
        self.exec(
            "CREATE TABLE IF NOT EXISTS skill_invocations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_name TEXT NOT NULL,
                executor TEXT NOT NULL,
                todo_id INTEGER,
                status TEXT DEFAULT 'invoked',
                duration_ms INTEGER,
                invoked_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc')),
                FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
            )",
        )
        .await?;

        // --- Indexes for frequently-filtered columns ---
        self.exec("CREATE INDEX IF NOT EXISTS idx_todos_deleted_at ON todos(deleted_at)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_todos_status ON todos(status)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_todos_task_id ON todos(task_id)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_execution_records_todo_id ON execution_records(todo_id)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_execution_records_task_id ON execution_records(task_id)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_execution_records_pid ON execution_records(pid)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_execution_records_session_id ON execution_records(session_id)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_execution_records_status ON execution_records(status)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_todo_tags_todo_id ON todo_tags(todo_id)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_skill_invocations_skill_name ON skill_invocations(skill_name)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_skill_invocations_executor ON skill_invocations(executor)").await?;
        self.exec("CREATE INDEX IF NOT EXISTS idx_skill_invocations_todo_id ON skill_invocations(todo_id)").await?;

        // Trigger: fill created_at with UTC time on INSERT if not set
        self.exec(
            "CREATE TRIGGER IF NOT EXISTS set_todos_created_at_utc AFTER INSERT ON todos
             WHEN new.created_at IS NULL OR new.created_at = ''
             BEGIN
                 UPDATE todos SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc') WHERE rowid = new.rowid;
             END",
        )
        .await?;

        self.exec(
            "CREATE TRIGGER IF NOT EXISTS set_todos_updated_at_utc BEFORE UPDATE ON todos
             WHEN new.updated_at IS NULL OR new.updated_at = ''
             BEGIN
                 SELECT raise(IGNORE);
             END",
        )
        .await?;

        self.exec(
            "CREATE TRIGGER IF NOT EXISTS set_tags_created_at_utc AFTER INSERT ON tags
             WHEN new.created_at IS NULL OR new.created_at = ''
             BEGIN
                 UPDATE tags SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc') WHERE rowid = new.rowid;
             END",
        )
        .await?;

        // Agent Bots table
        self.exec(
            "CREATE TABLE IF NOT EXISTS agent_bots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_type TEXT NOT NULL,
                bot_name TEXT NOT NULL,
                app_id TEXT NOT NULL,
                app_secret TEXT NOT NULL,
                bot_open_id TEXT,
                domain TEXT,
                enabled INTEGER DEFAULT 1,
                config TEXT DEFAULT '{}',
                created_at TEXT,
                updated_at TEXT
            )",
        )
        .await?;

        // Migrate: add config column if missing (existing databases)
        let cols = self
            .conn
            .query_all(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA table_info(agent_bots)".to_string(),
            ))
            .await
            .unwrap_or_default();
        let has_config = cols.iter().any(|row| {
            row.try_get::<String>("", "name").map(|n| n == "config").unwrap_or(false)
        });
        if !has_config {
            self.exec("ALTER TABLE agent_bots ADD COLUMN config TEXT DEFAULT '{}'")
                .await?;
        }

        // Feishu Homes table
        self.exec(
            "CREATE TABLE IF NOT EXISTS feishu_homes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_id INTEGER NOT NULL,
                user_open_id TEXT NOT NULL,
                chat_id TEXT,
                receive_id TEXT NOT NULL,
                receive_id_type TEXT NOT NULL,
                created_at TEXT,
                updated_at TEXT,
                FOREIGN KEY (bot_id) REFERENCES agent_bots(id),
                UNIQUE(bot_id, user_open_id)
            )",
        )
        .await?;

        // Feishu Messages table
        self.exec(
            "CREATE TABLE IF NOT EXISTS feishu_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_id INTEGER NOT NULL,
                message_id TEXT NOT NULL UNIQUE,
                chat_id TEXT NOT NULL,
                chat_type TEXT NOT NULL,
                sender_open_id TEXT NOT NULL,
                sender_nickname TEXT,
                sender_type TEXT,
                content TEXT,
                msg_type TEXT NOT NULL DEFAULT 'text',
                is_mention INTEGER DEFAULT 0,
                processed INTEGER DEFAULT 0,
                is_history INTEGER DEFAULT 0,
                fetch_time TEXT,
                created_at TEXT,
                FOREIGN KEY (bot_id) REFERENCES agent_bots(id)
            )",
        )
        .await?;

        // 添加 sender_nickname 字段的迁移（向后兼容）
        self.exec("ALTER TABLE feishu_messages ADD COLUMN IF NOT EXISTS sender_nickname TEXT")
            .await.ok();

        // 添加 sender_type 字段的迁移（向后兼容）
        self.exec("ALTER TABLE feishu_messages ADD COLUMN IF NOT EXISTS sender_type TEXT")
            .await.ok();

        // 添加 is_history 字段的迁移（向后兼容）
        self.exec("ALTER TABLE feishu_messages ADD COLUMN IF NOT EXISTS is_history INTEGER DEFAULT 0")
            .await.ok();

        // 添加 fetch_time 字段的迁移（向后兼容）
        self.exec("ALTER TABLE feishu_messages ADD COLUMN IF NOT EXISTS fetch_time TEXT")
            .await.ok();

        // 添加 processed_todo_id 字段的迁移（向后兼容）
        // 注意：SQLite 3.39.0+ 支持 IF NOT EXISTS，但旧版本不支持此语法
        // 先尝试带 IF NOT EXISTS 的版本，失败后再尝试不带 IF NOT EXISTS 的版本
        let add_result = self.exec("ALTER TABLE feishu_messages ADD COLUMN IF NOT EXISTS processed_todo_id INTEGER").await;
        if add_result.is_err() {
            // 尝试不带 IF NOT EXISTS 的版本（如果列已存在会报错，被 .ok() 忽略）
            self.exec("ALTER TABLE feishu_messages ADD COLUMN processed_todo_id INTEGER")
                .await
                .ok();
        }

        // 添加 execution_record_id 字段的迁移
        let add_exec_result = self.exec("ALTER TABLE feishu_messages ADD COLUMN IF NOT EXISTS execution_record_id INTEGER").await;
        if add_exec_result.is_err() {
            self.exec("ALTER TABLE feishu_messages ADD COLUMN execution_record_id INTEGER")
                .await
                .ok();
        }

        // Feishu History Chats table
        self.exec(
            "CREATE TABLE IF NOT EXISTS feishu_history_chats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_id INTEGER NOT NULL,
                chat_id TEXT NOT NULL,
                chat_name TEXT,
                enabled INTEGER DEFAULT 1,
                last_fetch_time TEXT,
                polling_interval_secs INTEGER DEFAULT 60,
                created_at TEXT,
                FOREIGN KEY (bot_id) REFERENCES agent_bots(id),
                UNIQUE(bot_id, chat_id)
            )",
        )
        .await?;

        // Feishu Push Targets — one row per bot, p2p and group IDs as separate fields
        self.exec("DROP TABLE IF EXISTS feishu_push_targets").await?;
        self.exec(
            "CREATE TABLE feishu_push_targets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_id INTEGER NOT NULL,
                p2p_receive_id TEXT NOT NULL DEFAULT '',
                group_chat_id TEXT NOT NULL DEFAULT '',
                receive_id_type TEXT NOT NULL DEFAULT 'open_id',
                push_level TEXT DEFAULT 'result_only',
                p2p_response_enabled INTEGER DEFAULT 1,
                group_response_enabled INTEGER DEFAULT 1,
                created_at TEXT,
                updated_at TEXT,
                FOREIGN KEY (bot_id) REFERENCES agent_bots(id)
            )",
        )
        .await?;

        // feishu_response_config 表（响应开关独立配置）
        self.exec("DROP TABLE IF EXISTS feishu_response_config").await?;
        self.exec(
            "CREATE TABLE feishu_response_config (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_id INTEGER NOT NULL,
                target_type TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                debounce_secs INTEGER DEFAULT 20,
                created_at TEXT,
                updated_at TEXT,
                UNIQUE(bot_id, target_type)
            )",
        )
        .await?;

        // Migrate: add debounce_secs column if missing
        let has_debounce: i64 = self.conn
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "SELECT COUNT(*) FROM pragma_table_info('feishu_response_config') WHERE name='debounce_secs'",
            ))
            .await?
            .map(|r| r.try_get::<i64>("", "COUNT(*)").unwrap_or(0))
            .unwrap_or(0);
        if has_debounce == 0 {
            self.exec("ALTER TABLE feishu_response_config ADD COLUMN debounce_secs INTEGER DEFAULT 20").await?;
        }

        // feishu_group_whitelist 表（群聊响应白名单）
        self.exec("DROP TABLE IF EXISTS feishu_group_whitelist").await?;
        self.exec(
            "CREATE TABLE feishu_group_whitelist (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_id INTEGER NOT NULL,
                sender_open_id TEXT NOT NULL,
                sender_name TEXT,
                created_at TEXT,
                UNIQUE(bot_id, sender_open_id)
            )",
        )
        .await?;

        Ok(())
    }
}

mod todo;
mod tag;
mod execution;
mod skills;
mod agent_bot;
mod feishu_home;
mod feishu_message;
mod feishu_push_target;
mod feishu_history_chat;
mod feishu_response_config;
mod feishu_group_whitelist;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Timelike};

    async fn setup_db() -> Database {
        Database::new(":memory:").await.unwrap()
    }

    fn parse_utc(ts: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc)
    }

    fn truncate_seconds(dt: DateTime<Utc>) -> DateTime<Utc> {
        dt.with_nanosecond(0).unwrap()
    }

    #[tokio::test]
    async fn test_todo_created_at_is_utc() {
        let db = setup_db().await;
        let before = truncate_seconds(Utc::now());
        let id = db.create_todo("Test", "Desc").await.unwrap();
        let after = truncate_seconds(Utc::now());

        let todo = db.get_todo(id).await.unwrap();
        let created = truncate_seconds(parse_utc(&todo.created_at));

        assert!(created >= before, "created_at should not be before test start");
        assert!(created <= after, "created_at should not be after test end");
        assert!(todo.created_at.ends_with('Z'), "UTC timestamp must end with Z");
    }

    #[tokio::test]
    async fn test_todo_updated_at_changes_on_update() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Desc").await.unwrap();
        let original = db.get_todo(id).await.unwrap().updated_at;

        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        db.update_todo_full(
            id,
            "Updated",
            "Desc",
            crate::models::TodoStatus::InProgress,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
        let updated = db.get_todo(id).await.unwrap().updated_at;

        assert_ne!(original, updated, "updated_at should change after update");
        assert!(updated.ends_with('Z'));
    }

    #[tokio::test]
    async fn test_todo_deleted_at_is_utc() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Desc").await.unwrap();
        let before = truncate_seconds(Utc::now());
        db.delete_todo(id).await.unwrap();
        let after = truncate_seconds(Utc::now());

        let model = entity::todos::Entity::find_by_id(id)
            .one(&db.conn)
            .await
            .unwrap()
            .unwrap();
        let deleted_at = model.deleted_at.unwrap();
        let dt = truncate_seconds(parse_utc(&deleted_at));
        assert!(dt >= before);
        assert!(dt <= after);
        assert!(deleted_at.ends_with('Z'));
    }

    #[tokio::test]
    async fn test_tag_created_at_is_utc() {
        let db = setup_db().await;
        let before = truncate_seconds(Utc::now());
        let id = db.create_tag("urgent", "#ff0000").await.unwrap();
        let after = truncate_seconds(Utc::now());

        let tag = db.get_tags().await.unwrap().into_iter().find(|t| t.id == id).unwrap();
        let created = truncate_seconds(parse_utc(&tag.created_at));

        assert!(created >= before);
        assert!(created <= after);
        assert!(tag.created_at.ends_with('Z'));
    }

    #[tokio::test]
    async fn test_execution_record_started_at_is_utc() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Desc").await.unwrap();
        let before = truncate_seconds(Utc::now());
        let record_id = db
            .create_execution_record(todo_id, "echo hi", "claudecode", "manual", "test-task-id", None, None)
            .await
            .unwrap();
        let after = truncate_seconds(Utc::now());

        let (records, _) = db.get_execution_records(todo_id, 100, 0).await;
        let record = records.into_iter().find(|r| r.id == record_id).unwrap();
        let started = truncate_seconds(parse_utc(&record.started_at));

        assert!(started >= before);
        assert!(started <= after);
        assert!(record.started_at.ends_with('Z'));
    }

    #[tokio::test]
    async fn test_execution_record_finished_at_is_utc() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Desc").await.unwrap();
        let record_id = db
            .create_execution_record(todo_id, "echo hi", "claudecode", "manual", "test-task-id", None, None)
            .await
            .unwrap();

        let before = truncate_seconds(Utc::now());
        db.update_execution_record(record_id, crate::models::ExecutionStatus::Success.as_str(), "[]", "done", None, None)
            .await
            .unwrap();
        let after = truncate_seconds(Utc::now());

        let (records, _) = db.get_execution_records(todo_id, 100, 0).await;
        let record = records.into_iter().find(|r| r.id == record_id).unwrap();
        let finished_at = record.finished_at.unwrap();
        let finished = truncate_seconds(parse_utc(&finished_at));

        assert!(finished >= before);
        assert!(finished <= after);
        assert!(finished_at.ends_with('Z'));
    }

    // ===== Todo CRUD tests =====

    #[tokio::test]
    async fn test_create_and_get_todo() {
        let db = setup_db().await;
        let id = db.create_todo("Title", "Prompt").await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.title, "Title");
        assert_eq!(todo.prompt, "Prompt");
        assert_eq!(todo.status, crate::models::TodoStatus::Pending);
        assert!(!todo.scheduler_enabled);
    }

    #[tokio::test]
    async fn test_get_todos_excludes_deleted() {
        let db = setup_db().await;
        let id = db.create_todo("Active", "Prompt").await.unwrap();
        db.delete_todo(id).await.unwrap();
        let todos = db.get_todos().await.unwrap();
        assert!(todos.iter().all(|t| t.id != id));
    }

    #[tokio::test]
    async fn test_get_todos_ordering() {
        let db = setup_db().await;
        let id1 = db.create_todo("First", "Prompt").await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let id2 = db.create_todo("Second", "Prompt").await.unwrap();
        let todos = db.get_todos().await.unwrap();
        assert_eq!(todos[0].id, id2);
        assert_eq!(todos[1].id, id1);
    }

    #[tokio::test]
    async fn test_update_todo_full() {
        let db = setup_db().await;
        let id = db.create_todo("Old", "Old prompt").await.unwrap();
        db.update_todo_full(
            id,
            "New",
            "New prompt",
            crate::models::TodoStatus::InProgress,
            Some("opencode"),
            Some(true),
            Some("0 0 * * *"),
            Some("/tmp/workspace"),
        )
        .await
        .unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.title, "New");
        assert_eq!(todo.prompt, "New prompt");
        assert_eq!(todo.status, crate::models::TodoStatus::InProgress);
        assert_eq!(todo.executor, Some("opencode".to_string()));
        assert!(todo.scheduler_enabled);
        assert_eq!(todo.scheduler_config, Some("0 0 * * *".to_string()));
        assert_eq!(todo.workspace, Some("/tmp/workspace".to_string()));
    }

    #[tokio::test]
    async fn test_update_todo_executor() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await.unwrap();
        db.update_todo_executor(id, "joinai").await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.executor, Some("joinai".to_string()));
    }

    #[tokio::test]
    async fn test_update_todo_task_id() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await.unwrap();
        db.update_todo_task_id(id, Some("task-123")).await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.task_id, Some("task-123".to_string()));
        db.update_todo_task_id(id, None).await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert!(todo.task_id.is_none());
    }

    #[tokio::test]
    async fn test_update_todo_scheduler() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await.unwrap();
        db.update_todo_scheduler(id, true, Some("0 0 * * *")).await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert!(todo.scheduler_enabled);
        assert_eq!(todo.scheduler_config, Some("0 0 * * *".to_string()));
    }

    #[tokio::test]
    async fn test_force_update_todo_status() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await.unwrap();
        db.force_update_todo_status(id, crate::models::TodoStatus::Failed).await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.status, crate::models::TodoStatus::Failed);
    }

    #[tokio::test]
    async fn test_delete_todo_soft_delete() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await.unwrap();
        db.delete_todo(id).await.unwrap();
        assert!(db.get_todo(id).await.is_none());
        let todos = db.get_todos().await.unwrap();
        assert!(todos.iter().all(|t| t.id != id));
    }

    #[tokio::test]
    async fn test_start_todo_execution() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await.unwrap();
        db.start_todo_execution(id, "task-1").await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.status, crate::models::TodoStatus::Running);
        assert_eq!(todo.task_id, Some("task-1".to_string()));
    }

    #[tokio::test]
    async fn test_finish_todo_execution_success() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await.unwrap();
        db.start_todo_execution(id, "task-1").await.unwrap();
        db.finish_todo_execution(id, true).await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.status, crate::models::TodoStatus::Completed);
        assert!(todo.task_id.is_none());
    }

    #[tokio::test]
    async fn test_finish_todo_execution_failure() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await.unwrap();
        db.start_todo_execution(id, "task-1").await.unwrap();
        db.finish_todo_execution(id, false).await.unwrap();
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.status, crate::models::TodoStatus::Failed);
    }

    #[tokio::test]
    async fn test_get_scheduler_todos() {
        let db = setup_db().await;
        let id1 = db.create_todo("Scheduled", "Prompt").await.unwrap();
        db.update_todo_scheduler(id1, true, Some("0 0 * * *")).await.unwrap();
        let id2 = db.create_todo("Normal", "Prompt").await.unwrap();
        let scheduled = db.get_scheduler_todos().await.unwrap();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].id, id1);
        assert!(scheduled.iter().all(|t| t.id != id2));
    }

    #[tokio::test]
    async fn test_todo_with_tag_ids() {
        let db = setup_db().await;
        let tag_id = db.create_tag("urgent", "#ff0000").await.unwrap();
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        db.add_todo_tag(todo_id, tag_id).await;
        let todo = db.get_todo(todo_id).await.unwrap();
        assert_eq!(todo.tag_ids, vec![tag_id]);
    }

    // ===== Tag CRUD tests =====

    #[tokio::test]
    async fn test_create_and_get_tag() {
        let db = setup_db().await;
        let id = db.create_tag("urgent", "#ff0000").await.unwrap();
        let tags = db.get_tags().await.unwrap();
        let tag = tags.iter().find(|t| t.id == id).unwrap();
        assert_eq!(tag.name, "urgent");
        assert_eq!(tag.color, "#ff0000");
    }

    #[tokio::test]
    async fn test_get_tags_ordered_by_name() {
        let db = setup_db().await;
        db.create_tag("zebra", "#000").await.unwrap();
        db.create_tag("apple", "#fff").await.unwrap();
        db.create_tag("mango", "#aaa").await.unwrap();
        let tags = db.get_tags().await.unwrap();
        assert_eq!(tags[0].name, "apple");
        assert_eq!(tags[1].name, "mango");
        assert_eq!(tags[2].name, "zebra");
    }

    #[tokio::test]
    async fn test_delete_tag() {
        let db = setup_db().await;
        let id = db.create_tag("temp", "#000").await.unwrap();
        db.delete_tag(id).await;
        let tags = db.get_tags().await.unwrap();
        assert!(tags.iter().all(|t| t.id != id));
    }

    #[tokio::test]
    async fn test_add_todo_tag() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let tag_id = db.create_tag("urgent", "#ff0000").await.unwrap();
        db.add_todo_tag(todo_id, tag_id).await;
        let todo = db.get_todo(todo_id).await.unwrap();
        assert_eq!(todo.tag_ids, vec![tag_id]);
    }

    #[tokio::test]
    async fn test_add_todo_tag_duplicate_ignored() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let tag_id = db.create_tag("urgent", "#ff0000").await.unwrap();
        db.add_todo_tag(todo_id, tag_id).await;
        db.add_todo_tag(todo_id, tag_id).await; // should not panic
        let todo = db.get_todo(todo_id).await.unwrap();
        assert_eq!(todo.tag_ids, vec![tag_id]);
    }

    #[tokio::test]
    async fn test_set_todo_tags_replace_all() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let tag1 = db.create_tag("a", "#000").await.unwrap();
        let tag2 = db.create_tag("b", "#fff").await.unwrap();
        let tag3 = db.create_tag("c", "#aaa").await.unwrap();
        db.add_todo_tag(todo_id, tag1).await;
        db.set_todo_tags(todo_id, &[tag2, tag3]).await;
        let todo = db.get_todo(todo_id).await.unwrap();
        assert_eq!(todo.tag_ids.len(), 2);
        assert!(todo.tag_ids.contains(&tag2));
        assert!(todo.tag_ids.contains(&tag3));
        assert!(!todo.tag_ids.contains(&tag1));
    }

    #[tokio::test]
    async fn test_set_todo_tags_empty_clears_all() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let tag_id = db.create_tag("urgent", "#ff0000").await.unwrap();
        db.add_todo_tag(todo_id, tag_id).await;
        db.set_todo_tags(todo_id, &[]).await;
        let todo = db.get_todo(todo_id).await.unwrap();
        assert!(todo.tag_ids.is_empty());
    }

    #[tokio::test]
    async fn test_delete_todo_cascades_tags() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let tag_id = db.create_tag("urgent", "#ff0000").await.unwrap();
        db.add_todo_tag(todo_id, tag_id).await;
        db.delete_todo(todo_id).await.unwrap();
        // tag should still exist but association should be gone
        let tags = db.get_tags().await.unwrap();
        assert!(tags.iter().any(|t| t.id == tag_id));
    }

    // ===== Execution record tests =====

    #[tokio::test]
    async fn test_create_execution_record() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let record_id = db.create_execution_record(todo_id, "echo hi", "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        let (records, total) = db.get_execution_records(todo_id, 100, 0).await;
        assert_eq!(total, 1);
        let record = records.iter().find(|r| r.id == record_id).unwrap();
        assert_eq!(record.status, crate::models::ExecutionStatus::Running);
        assert_eq!(record.command, "echo hi");
        assert_eq!(record.executor, Some("claudecode".to_string()));
        assert_eq!(record.trigger_type, "manual");
        assert!(record.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_get_execution_records_pagination() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        for i in 0..5 {
            db.create_execution_record(todo_id, &format!("cmd{}", i), "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        }
        let (records, total) = db.get_execution_records(todo_id, 2, 0).await;
        assert_eq!(total, 5);
        assert_eq!(records.len(), 2);
    }

    #[tokio::test]
    async fn test_get_execution_records_offset() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        for i in 0..3 {
            db.create_execution_record(todo_id, &format!("cmd{}", i), "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        }
        let (records, total) = db.get_execution_records(todo_id, 10, 2).await;
        assert_eq!(total, 3);
        assert_eq!(records.len(), 1);
    }

    #[tokio::test]
    async fn test_update_execution_record() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let record_id = db.create_execution_record(todo_id, "echo hi", "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        let usage = crate::models::ExecutionUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: Some(20),
            cache_creation_input_tokens: None,
            total_cost_usd: Some(0.005),
            duration_ms: Some(1000),
        };
        db.update_execution_record(record_id, "success", "[{\"type\":\"info\"}]", "done", Some(&usage), Some("claude-3")).await.unwrap();
        let (records, _) = db.get_execution_records(todo_id, 100, 0).await;
        let record = records.iter().find(|r| r.id == record_id).unwrap();
        assert_eq!(record.status, crate::models::ExecutionStatus::Success);
        assert_eq!(record.logs, "[{\"type\":\"info\"}]");
        assert_eq!(record.result, Some("done".to_string()));
        assert_eq!(record.model, Some("claude-3".to_string()));
        assert!(record.finished_at.is_some());
        let record_usage = record.usage.as_ref().unwrap();
        assert_eq!(record_usage.input_tokens, 100);
        assert_eq!(record_usage.output_tokens, 50);
    }

    #[tokio::test]
    async fn test_get_execution_summary_empty() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let summary = db.get_execution_summary(todo_id).await;
        assert_eq!(summary.todo_id, todo_id);
        assert_eq!(summary.total_executions, 0);
        assert_eq!(summary.success_count, 0);
        assert_eq!(summary.failed_count, 0);
        assert_eq!(summary.running_count, 0);
        assert!(summary.total_cost_usd.is_none());
    }

    #[tokio::test]
    async fn test_get_execution_summary_counts() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let r1 = db.create_execution_record(todo_id, "cmd1", "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        db.update_execution_record(r1, "success", "[]", "", None, None).await.unwrap();
        let r2 = db.create_execution_record(todo_id, "cmd2", "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        db.update_execution_record(r2, "failed", "[]", "", None, None).await.unwrap();
        let _r3 = db.create_execution_record(todo_id, "cmd3", "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        // r3 stays "running"
        let summary = db.get_execution_summary(todo_id).await;
        assert_eq!(summary.total_executions, 3);
        assert_eq!(summary.success_count, 1);
        assert_eq!(summary.failed_count, 1);
        assert_eq!(summary.running_count, 1);
    }

    #[tokio::test]
    async fn test_get_execution_summary_tokens_and_cost() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await.unwrap();
        let r1 = db.create_execution_record(todo_id, "cmd1", "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        let usage1 = crate::models::ExecutionUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: Some(20),
            cache_creation_input_tokens: Some(10),
            total_cost_usd: Some(0.005),
            duration_ms: Some(1000),
        };
        db.update_execution_record(r1, "success", "[]", "", Some(&usage1), None).await.unwrap();
        let r2 = db.create_execution_record(todo_id, "cmd2", "claudecode", "manual", "test-task-id", None, None).await.unwrap();
        let usage2 = crate::models::ExecutionUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            total_cost_usd: Some(0.010),
            duration_ms: Some(2000),
        };
        db.update_execution_record(r2, "success", "[]", "", Some(&usage2), None).await.unwrap();
        let summary = db.get_execution_summary(todo_id).await;
        assert_eq!(summary.total_input_tokens, 300);
        assert_eq!(summary.total_output_tokens, 150);
        assert_eq!(summary.total_cache_read_tokens, 20);
        assert_eq!(summary.total_cache_creation_tokens, 10);
        assert_eq!(summary.total_cost_usd, Some(0.015));
    }

}
