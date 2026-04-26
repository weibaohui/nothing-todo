//! Database access layer (SeaORM).
//!
//! - Fixed database path: `~/.ntd/data.db`
//! - Built-in SQLite (libsqlite3-sys/bundled), no system dependencies
//! - All public methods are async

use std::str::FromStr;
use std::time::Duration;

use chrono::Utc;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database as SeaDatabase, DatabaseConnection, DbBackend,
    EntityTrait, Statement,
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
        db.init_tables().await?;
        Ok(db)
    }

    pub(super) async fn exec(&self, sql: &str) -> Result<(), sea_orm::DbErr> {
        self.conn
            .execute(Statement::from_string(DbBackend::Sqlite, sql.to_string()))
            .await
            .map(|_| ())
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
                task_id TEXT
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
                FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
            )",
        )
        .await?;

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

        Ok(())
    }
}

mod todo;
mod tag;
mod execution;

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
        let id = db.create_todo("Test", "Desc").await;
        let after = truncate_seconds(Utc::now());

        let todo = db.get_todo(id).await.unwrap();
        let created = parse_utc(&todo.created_at);

        assert!(created >= before, "created_at should not be before test start");
        assert!(created <= after, "created_at should not be after test end");
        assert!(todo.created_at.ends_with('Z'), "UTC timestamp must end with Z");
    }

    #[tokio::test]
    async fn test_todo_updated_at_changes_on_update() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Desc").await;
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
        )
        .await;
        let updated = db.get_todo(id).await.unwrap().updated_at;

        assert_ne!(original, updated, "updated_at should change after update");
        assert!(updated.ends_with('Z'));
    }

    #[tokio::test]
    async fn test_todo_deleted_at_is_utc() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Desc").await;
        let before = truncate_seconds(Utc::now());
        db.delete_todo(id).await;
        let after = truncate_seconds(Utc::now());

        let model = entity::todos::Entity::find_by_id(id)
            .one(&db.conn)
            .await
            .unwrap()
            .unwrap();
        let deleted_at = model.deleted_at.unwrap();
        let dt = parse_utc(&deleted_at);
        assert!(dt >= before);
        assert!(dt <= after);
        assert!(deleted_at.ends_with('Z'));
    }

    #[tokio::test]
    async fn test_tag_created_at_is_utc() {
        let db = setup_db().await;
        let before = truncate_seconds(Utc::now());
        let id = db.create_tag("urgent", "#ff0000").await;
        let after = truncate_seconds(Utc::now());

        let tag = db.get_tags().await.into_iter().find(|t| t.id == id).unwrap();
        let created = parse_utc(&tag.created_at);

        assert!(created >= before);
        assert!(created <= after);
        assert!(tag.created_at.ends_with('Z'));
    }

    #[tokio::test]
    async fn test_execution_record_started_at_is_utc() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Desc").await;
        let before = truncate_seconds(Utc::now());
        let record_id = db
            .create_execution_record(todo_id, "echo hi", "claudecode", "manual")
            .await;
        let after = truncate_seconds(Utc::now());

        let (records, _) = db.get_execution_records(todo_id, 100, 0).await;
        let record = records.into_iter().find(|r| r.id == record_id).unwrap();
        let started = parse_utc(&record.started_at);

        assert!(started >= before);
        assert!(started <= after);
        assert!(record.started_at.ends_with('Z'));
    }

    #[tokio::test]
    async fn test_execution_record_finished_at_is_utc() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Desc").await;
        let record_id = db
            .create_execution_record(todo_id, "echo hi", "claudecode", "manual")
            .await;

        let before = truncate_seconds(Utc::now());
        db.update_execution_record(record_id, "success", "[]", "done")
            .await;
        let after = truncate_seconds(Utc::now());

        let (records, _) = db.get_execution_records(todo_id, 100, 0).await;
        let record = records.into_iter().find(|r| r.id == record_id).unwrap();
        let finished_at = record.finished_at.unwrap();
        let finished = parse_utc(&finished_at);

        assert!(finished >= before);
        assert!(finished <= after);
        assert!(finished_at.ends_with('Z'));
    }
}
