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
        db.init_tables().await?;
        Ok(db)
    }

    pub(super) async fn exec(&self, sql: &str) -> Result<(), sea_orm::DbErr> {
        self.conn
            .execute(Statement::from_string(DbBackend::Sqlite, sql.to_string()))
            .await
            .map(|_| ())
    }

    pub(super) async fn exec_update<M>(&self, model: M)
    where
        M: ActiveModelTrait + ActiveModelBehavior + Send,
        <<M as ActiveModelTrait>::Entity as EntityTrait>::Model: IntoActiveModel<M>,
    {
        if let Err(e) = model.update(&self.conn).await {
            tracing::error!("Database update failed: {}", e);
        }
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
        let created = truncate_seconds(parse_utc(&todo.created_at));

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
        let dt = truncate_seconds(parse_utc(&deleted_at));
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
        let created = truncate_seconds(parse_utc(&tag.created_at));

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
        let started = truncate_seconds(parse_utc(&record.started_at));

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
        db.update_execution_record(record_id, crate::models::ExecutionStatus::Success.as_str(), "[]", "done", None, None)
            .await;
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
        let id = db.create_todo("Title", "Prompt").await;
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.title, "Title");
        assert_eq!(todo.prompt, "Prompt");
        assert_eq!(todo.status, crate::models::TodoStatus::Pending);
        assert!(!todo.scheduler_enabled);
    }

    #[tokio::test]
    async fn test_get_todos_excludes_deleted() {
        let db = setup_db().await;
        let id = db.create_todo("Active", "Prompt").await;
        db.delete_todo(id).await;
        let todos = db.get_todos().await;
        assert!(todos.iter().all(|t| t.id != id));
    }

    #[tokio::test]
    async fn test_get_todos_ordering() {
        let db = setup_db().await;
        let id1 = db.create_todo("First", "Prompt").await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let id2 = db.create_todo("Second", "Prompt").await;
        let todos = db.get_todos().await;
        assert_eq!(todos[0].id, id2);
        assert_eq!(todos[1].id, id1);
    }

    #[tokio::test]
    async fn test_update_todo_full() {
        let db = setup_db().await;
        let id = db.create_todo("Old", "Old prompt").await;
        db.update_todo_full(
            id,
            "New",
            "New prompt",
            crate::models::TodoStatus::InProgress,
            Some("opencode"),
            Some(true),
            Some("0 0 * * *"),
        )
        .await;
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.title, "New");
        assert_eq!(todo.prompt, "New prompt");
        assert_eq!(todo.status, crate::models::TodoStatus::InProgress);
        assert_eq!(todo.executor, Some("opencode".to_string()));
        assert!(todo.scheduler_enabled);
        assert_eq!(todo.scheduler_config, Some("0 0 * * *".to_string()));
    }

    #[tokio::test]
    async fn test_update_todo_executor() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await;
        db.update_todo_executor(id, "joinai").await;
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.executor, Some("joinai".to_string()));
    }

    #[tokio::test]
    async fn test_update_todo_task_id() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await;
        db.update_todo_task_id(id, Some("task-123")).await;
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.task_id, Some("task-123".to_string()));
        db.update_todo_task_id(id, None).await;
        let todo = db.get_todo(id).await.unwrap();
        assert!(todo.task_id.is_none());
    }

    #[tokio::test]
    async fn test_update_todo_scheduler() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await;
        db.update_todo_scheduler(id, true, Some("0 0 * * *")).await;
        let todo = db.get_todo(id).await.unwrap();
        assert!(todo.scheduler_enabled);
        assert_eq!(todo.scheduler_config, Some("0 0 * * *".to_string()));
    }

    #[tokio::test]
    async fn test_force_update_todo_status() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await;
        db.force_update_todo_status(id, crate::models::TodoStatus::Failed).await;
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.status, crate::models::TodoStatus::Failed);
    }

    #[tokio::test]
    async fn test_delete_todo_soft_delete() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await;
        db.delete_todo(id).await;
        assert!(db.get_todo(id).await.is_none());
        let todos = db.get_todos().await;
        assert!(todos.iter().all(|t| t.id != id));
    }

    #[tokio::test]
    async fn test_start_todo_execution() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await;
        db.start_todo_execution(id, "task-1").await;
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.status, crate::models::TodoStatus::Running);
        assert_eq!(todo.task_id, Some("task-1".to_string()));
    }

    #[tokio::test]
    async fn test_finish_todo_execution_success() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await;
        db.start_todo_execution(id, "task-1").await;
        db.finish_todo_execution(id, true).await;
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.status, crate::models::TodoStatus::Completed);
        assert!(todo.task_id.is_none());
    }

    #[tokio::test]
    async fn test_finish_todo_execution_failure() {
        let db = setup_db().await;
        let id = db.create_todo("Test", "Prompt").await;
        db.start_todo_execution(id, "task-1").await;
        db.finish_todo_execution(id, false).await;
        let todo = db.get_todo(id).await.unwrap();
        assert_eq!(todo.status, crate::models::TodoStatus::Failed);
    }

    #[tokio::test]
    async fn test_get_scheduler_todos() {
        let db = setup_db().await;
        let id1 = db.create_todo("Scheduled", "Prompt").await;
        db.update_todo_scheduler(id1, true, Some("0 0 * * *")).await;
        let id2 = db.create_todo("Normal", "Prompt").await;
        let scheduled = db.get_scheduler_todos().await;
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].id, id1);
        assert!(scheduled.iter().all(|t| t.id != id2));
    }

    #[tokio::test]
    async fn test_todo_with_tag_ids() {
        let db = setup_db().await;
        let tag_id = db.create_tag("urgent", "#ff0000").await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        db.add_todo_tag(todo_id, tag_id).await;
        let todo = db.get_todo(todo_id).await.unwrap();
        assert_eq!(todo.tag_ids, vec![tag_id]);
    }

    // ===== Tag CRUD tests =====

    #[tokio::test]
    async fn test_create_and_get_tag() {
        let db = setup_db().await;
        let id = db.create_tag("urgent", "#ff0000").await;
        let tags = db.get_tags().await;
        let tag = tags.iter().find(|t| t.id == id).unwrap();
        assert_eq!(tag.name, "urgent");
        assert_eq!(tag.color, "#ff0000");
    }

    #[tokio::test]
    async fn test_get_tags_ordered_by_name() {
        let db = setup_db().await;
        db.create_tag("zebra", "#000").await;
        db.create_tag("apple", "#fff").await;
        db.create_tag("mango", "#aaa").await;
        let tags = db.get_tags().await;
        assert_eq!(tags[0].name, "apple");
        assert_eq!(tags[1].name, "mango");
        assert_eq!(tags[2].name, "zebra");
    }

    #[tokio::test]
    async fn test_delete_tag() {
        let db = setup_db().await;
        let id = db.create_tag("temp", "#000").await;
        db.delete_tag(id).await;
        let tags = db.get_tags().await;
        assert!(tags.iter().all(|t| t.id != id));
    }

    #[tokio::test]
    async fn test_add_todo_tag() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        let tag_id = db.create_tag("urgent", "#ff0000").await;
        db.add_todo_tag(todo_id, tag_id).await;
        let todo = db.get_todo(todo_id).await.unwrap();
        assert_eq!(todo.tag_ids, vec![tag_id]);
    }

    #[tokio::test]
    async fn test_add_todo_tag_duplicate_ignored() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        let tag_id = db.create_tag("urgent", "#ff0000").await;
        db.add_todo_tag(todo_id, tag_id).await;
        db.add_todo_tag(todo_id, tag_id).await; // should not panic
        let todo = db.get_todo(todo_id).await.unwrap();
        assert_eq!(todo.tag_ids, vec![tag_id]);
    }

    #[tokio::test]
    async fn test_set_todo_tags_replace_all() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        let tag1 = db.create_tag("a", "#000").await;
        let tag2 = db.create_tag("b", "#fff").await;
        let tag3 = db.create_tag("c", "#aaa").await;
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
        let todo_id = db.create_todo("Test", "Prompt").await;
        let tag_id = db.create_tag("urgent", "#ff0000").await;
        db.add_todo_tag(todo_id, tag_id).await;
        db.set_todo_tags(todo_id, &[]).await;
        let todo = db.get_todo(todo_id).await.unwrap();
        assert!(todo.tag_ids.is_empty());
    }

    #[tokio::test]
    async fn test_delete_todo_cascades_tags() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        let tag_id = db.create_tag("urgent", "#ff0000").await;
        db.add_todo_tag(todo_id, tag_id).await;
        db.delete_todo(todo_id).await;
        // tag should still exist but association should be gone
        let tags = db.get_tags().await;
        assert!(tags.iter().any(|t| t.id == tag_id));
    }

    // ===== Execution record tests =====

    #[tokio::test]
    async fn test_create_execution_record() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        let record_id = db.create_execution_record(todo_id, "echo hi", "claudecode", "manual").await;
        let (records, total) = db.get_execution_records(todo_id, 100, 0).await;
        assert_eq!(total, 1);
        let record = records.iter().find(|r| r.id == record_id).unwrap();
        assert_eq!(record.status, "running");
        assert_eq!(record.command, "echo hi");
        assert_eq!(record.executor, Some("claudecode".to_string()));
        assert_eq!(record.trigger_type, "manual");
        assert!(record.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_get_execution_records_pagination() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        for i in 0..5 {
            db.create_execution_record(todo_id, &format!("cmd{}", i), "claudecode", "manual").await;
        }
        let (records, total) = db.get_execution_records(todo_id, 2, 0).await;
        assert_eq!(total, 5);
        assert_eq!(records.len(), 2);
    }

    #[tokio::test]
    async fn test_get_execution_records_offset() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        for i in 0..3 {
            db.create_execution_record(todo_id, &format!("cmd{}", i), "claudecode", "manual").await;
        }
        let (records, total) = db.get_execution_records(todo_id, 10, 2).await;
        assert_eq!(total, 3);
        assert_eq!(records.len(), 1);
    }

    #[tokio::test]
    async fn test_update_execution_record() {
        let db = setup_db().await;
        let todo_id = db.create_todo("Test", "Prompt").await;
        let record_id = db.create_execution_record(todo_id, "echo hi", "claudecode", "manual").await;
        let usage = crate::models::ExecutionUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: Some(20),
            cache_creation_input_tokens: None,
            total_cost_usd: Some(0.005),
            duration_ms: Some(1000),
        };
        db.update_execution_record(record_id, "success", "[{\"type\":\"info\"}]", "done", Some(&usage), Some("claude-3")).await;
        let (records, _) = db.get_execution_records(todo_id, 100, 0).await;
        let record = records.iter().find(|r| r.id == record_id).unwrap();
        assert_eq!(record.status, "success");
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
        let todo_id = db.create_todo("Test", "Prompt").await;
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
        let todo_id = db.create_todo("Test", "Prompt").await;
        let r1 = db.create_execution_record(todo_id, "cmd1", "claudecode", "manual").await;
        db.update_execution_record(r1, "success", "[]", "", None, None).await;
        let r2 = db.create_execution_record(todo_id, "cmd2", "claudecode", "manual").await;
        db.update_execution_record(r2, "failed", "[]", "", None, None).await;
        let r3 = db.create_execution_record(todo_id, "cmd3", "claudecode", "manual").await;
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
        let todo_id = db.create_todo("Test", "Prompt").await;
        let r1 = db.create_execution_record(todo_id, "cmd1", "claudecode", "manual").await;
        let usage1 = crate::models::ExecutionUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: Some(20),
            cache_creation_input_tokens: Some(10),
            total_cost_usd: Some(0.005),
            duration_ms: Some(1000),
        };
        db.update_execution_record(r1, "success", "[]", "", Some(&usage1), None).await;
        let r2 = db.create_execution_record(todo_id, "cmd2", "claudecode", "manual").await;
        let usage2 = crate::models::ExecutionUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            total_cost_usd: Some(0.010),
            duration_ms: Some(2000),
        };
        db.update_execution_record(r2, "success", "[]", "", Some(&usage2), None).await;
        let summary = db.get_execution_summary(todo_id).await;
        assert_eq!(summary.total_input_tokens, 300);
        assert_eq!(summary.total_output_tokens, 150);
        assert_eq!(summary.total_cache_read_tokens, 20);
        assert_eq!(summary.total_cache_creation_tokens, 10);
        assert_eq!(summary.total_cost_usd, Some(0.015));
    }

}
