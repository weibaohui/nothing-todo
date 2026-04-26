// 注意: 原 rusqlite 实现已迁移到 SeaORM。原代码在 git 历史中可查阅。
// 本文件保持 Database 结构体的公开方法语义不变,所有方法改为 async。
//
// 数据库路径固定: ~/.ntd/data.db
// 使用 sqlx-sqlite + libsqlite3-sys (bundled) 内置 SQLite,不依赖系统库。

use std::str::FromStr;
use std::time::Duration;

use chrono::Utc;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectOptions, ConnectionTrait,
    Database as SeaDatabase, DatabaseConnection, DbBackend, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Statement,
};

use crate::models::{ExecutionRecord, ExecutionSummary, ExecutionUsage, Tag, Todo};

pub mod entity;
pub use entity::prelude::*;
use entity::{execution_records, tags, todo_tags, todos};

fn compute_next_run(cron_expr: &str) -> Option<String> {
    cron::Schedule::from_str(cron_expr).ok().and_then(|schedule| {
        schedule
            .upcoming(Utc)
            .next()
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
    })
}

fn now_utc() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub type DbPool = DatabaseConnection;

pub struct Database {
    conn: DatabaseConnection,
}

impl Database {
    /// 打开数据库连接 (异步)。
    /// path: 数据库文件路径 或 ":memory:"。
    pub async fn new(path: &str) -> Result<Self, sea_orm::DbErr> {
        let url = if path == ":memory:" {
            // 共享内存数据库,所有连接共用同一份内存
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

    pub fn pool(&self) -> &DatabaseConnection {
        &self.conn
    }

    async fn exec(&self, sql: &str) -> Result<(), sea_orm::DbErr> {
        self.conn
            .execute(Statement::from_string(DbBackend::Sqlite, sql.to_string()))
            .await
            .map(|_| ())
    }

    async fn exec_ignore(&self, sql: &str) {
        let _ = self.exec(sql).await;
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
                deleted_at TEXT
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

        // 历史 schema 兼容: 这些 ALTER 在已存在列时会报错,忽略即可
        self.exec_ignore("ALTER TABLE todos ADD COLUMN executor TEXT DEFAULT 'claudecode'")
            .await;
        self.exec_ignore("ALTER TABLE todos ADD COLUMN scheduler_enabled INTEGER DEFAULT 0")
            .await;
        self.exec_ignore("ALTER TABLE todos ADD COLUMN scheduler_config TEXT").await;
        self.exec_ignore("ALTER TABLE todos ADD COLUMN model TEXT").await;
        self.exec_ignore("ALTER TABLE todos ADD COLUMN task_id TEXT").await;

        self.exec_ignore(
            "CREATE TRIGGER IF NOT EXISTS set_todos_created_at_utc AFTER INSERT ON todos
             WHEN new.created_at IS NULL OR new.created_at = ''
             BEGIN
                 UPDATE todos SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc') WHERE rowid = new.rowid;
             END",
        )
        .await;

        self.exec_ignore(
            "CREATE TRIGGER IF NOT EXISTS set_todos_updated_at_utc BEFORE UPDATE ON todos
             WHEN new.updated_at IS NULL OR new.updated_at = ''
             BEGIN
                 SELECT raise(IGNORE);
             END",
        )
        .await;

        self.exec_ignore(
            "CREATE TRIGGER IF NOT EXISTS set_tags_created_at_utc AFTER INSERT ON tags
             WHEN new.created_at IS NULL OR new.created_at = ''
             BEGIN
                 UPDATE tags SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc') WHERE rowid = new.rowid;
             END",
        )
        .await;

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

        self.exec_ignore("ALTER TABLE execution_records ADD COLUMN usage TEXT").await;
        self.exec_ignore("ALTER TABLE execution_records ADD COLUMN executor TEXT").await;
        self.exec_ignore("ALTER TABLE execution_records ADD COLUMN model TEXT").await;
        self.exec_ignore(
            "ALTER TABLE execution_records ADD COLUMN trigger_type TEXT DEFAULT 'manual'",
        )
        .await;

        // 旧 description -> prompt 重命名 (老 db 兼容)
        self.exec_ignore("ALTER TABLE todos RENAME COLUMN description TO prompt").await;

        Ok(())
    }

    fn model_to_todo(m: todos::Model, tag_ids: Vec<i64>) -> Todo {
        let scheduler_enabled = m.scheduler_enabled.unwrap_or(0) != 0;
        let scheduler_config = m.scheduler_config.clone();
        let scheduler_next_run_at = if scheduler_enabled {
            scheduler_config.as_deref().and_then(compute_next_run)
        } else {
            None
        };
        Todo {
            id: m.id,
            title: m.title,
            prompt: m.prompt.unwrap_or_default(),
            status: m.status.unwrap_or_default(),
            created_at: m.created_at.unwrap_or_default(),
            updated_at: m.updated_at.unwrap_or_default(),
            tag_ids,
            executor: m.executor,
            scheduler_enabled,
            scheduler_config,
            scheduler_next_run_at,
            task_id: m.task_id,
        }
    }

    async fn fetch_tag_ids_for(&self, todo_id: i64) -> Vec<i64> {
        todo_tags::Entity::find()
            .filter(todo_tags::Column::TodoId.eq(todo_id))
            .all(&self.conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|t| t.tag_id)
            .collect()
    }

    // ===== Todo operations =====

    pub async fn get_todos(&self) -> Vec<Todo> {
        let models = todos::Entity::find()
            .filter(todos::Column::DeletedAt.is_null())
            .order_by_desc(todos::Column::UpdatedAt)
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let mut result = Vec::with_capacity(models.len());
        for m in models {
            let tag_ids = self.fetch_tag_ids_for(m.id).await;
            result.push(Self::model_to_todo(m, tag_ids));
        }
        result
    }

    pub async fn create_todo(&self, title: &str, prompt: &str) -> i64 {
        let now = now_utc();
        let am = todos::ActiveModel {
            title: ActiveValue::Set(title.to_string()),
            prompt: ActiveValue::Set(Some(prompt.to_string())),
            status: ActiveValue::Set(Some("pending".to_string())),
            created_at: ActiveValue::Set(Some(now.clone())),
            updated_at: ActiveValue::Set(Some(now)),
            executor: ActiveValue::Set(Some("claudecode".to_string())),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await.expect("insert todo failed");
        inserted.id
    }

    pub async fn update_todo(&self, id: i64, title: &str, prompt: &str, status: &str) {
        let now = now_utc();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            title: ActiveValue::Set(title.to_string()),
            prompt: ActiveValue::Set(Some(prompt.to_string())),
            status: ActiveValue::Set(Some(status.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_todo_full(
        &self,
        id: i64,
        title: &str,
        prompt: &str,
        status: &str,
        executor: Option<&str>,
        scheduler_enabled: Option<bool>,
        scheduler_config: Option<&str>,
    ) {
        let now = now_utc();
        let mut am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            title: ActiveValue::Set(title.to_string()),
            prompt: ActiveValue::Set(Some(prompt.to_string())),
            status: ActiveValue::Set(Some(status.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        if let Some(exec) = executor {
            am.executor = ActiveValue::Set(Some(exec.to_string()));
        }
        if let Some(enabled) = scheduler_enabled {
            am.scheduler_enabled = ActiveValue::Set(Some(if enabled { 1 } else { 0 }));
        }
        // 原行为: 只要 Option 是 Some(...) 就写 (包括 Some("")) 。新接口里 scheduler_config 来自调用方
        // 我们以 "Some => Set" 的语义实现
        if let Some(cfg) = scheduler_config {
            am.scheduler_config = ActiveValue::Set(Some(cfg.to_string()));
        }
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_todo_executor(&self, id: i64, executor: &str) {
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            executor: ActiveValue::Set(Some(executor.to_string())),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_todo_task_id(&self, id: i64, task_id: Option<&str>) {
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            task_id: ActiveValue::Set(task_id.map(|s| s.to_string())),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_todo_scheduler(&self, id: i64, enabled: bool, config: Option<&str>) {
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            scheduler_enabled: ActiveValue::Set(Some(if enabled { 1 } else { 0 })),
            scheduler_config: ActiveValue::Set(config.map(|s| s.to_string())),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn force_update_todo_status(&self, id: i64, status: &str) {
        let now = now_utc();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn delete_todo(&self, id: i64) {
        let now = now_utc();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(id),
            deleted_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn get_todo(&self, id: i64) -> Option<Todo> {
        let model = todos::Entity::find_by_id(id)
            .filter(todos::Column::DeletedAt.is_null())
            .one(&self.conn)
            .await
            .ok()
            .flatten()?;
        let tag_ids = self.fetch_tag_ids_for(id).await;
        Some(Self::model_to_todo(model, tag_ids))
    }

    pub async fn get_scheduler_todos(&self) -> Vec<Todo> {
        let models = todos::Entity::find()
            .filter(todos::Column::DeletedAt.is_null())
            .filter(todos::Column::SchedulerConfig.is_not_null())
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let mut result = Vec::with_capacity(models.len());
        for m in models {
            let tag_ids = self.fetch_tag_ids_for(m.id).await;
            result.push(Self::model_to_todo(m, tag_ids));
        }
        result
    }

    // ===== Tag operations =====

    pub async fn get_tags(&self) -> Vec<Tag> {
        tags::Entity::find()
            .order_by_asc(tags::Column::Name)
            .all(&self.conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| Tag {
                id: m.id,
                name: m.name,
                color: m.color.unwrap_or_default(),
                created_at: m.created_at.unwrap_or_default(),
            })
            .collect()
    }

    pub async fn create_tag(&self, name: &str, color: &str) -> i64 {
        let now = now_utc();
        let am = tags::ActiveModel {
            name: ActiveValue::Set(name.to_string()),
            color: ActiveValue::Set(Some(color.to_string())),
            created_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await.expect("insert tag failed");
        inserted.id
    }

    pub async fn delete_tag(&self, id: i64) {
        let _ = tags::Entity::delete_by_id(id).exec(&self.conn).await;
    }

    pub async fn add_todo_tag(&self, todo_id: i64, tag_id: i64) {
        let am = todo_tags::ActiveModel {
            todo_id: ActiveValue::Set(todo_id),
            tag_id: ActiveValue::Set(tag_id),
        };
        let _ = todo_tags::Entity::insert(am)
            .on_conflict(
                OnConflict::columns([todo_tags::Column::TodoId, todo_tags::Column::TagId])
                    .do_nothing()
                    .to_owned(),
            )
            .exec(&self.conn)
            .await;
    }

    pub async fn remove_todo_tags(&self, todo_id: i64) {
        let _ = todo_tags::Entity::delete_many()
            .filter(todo_tags::Column::TodoId.eq(todo_id))
            .exec(&self.conn)
            .await;
    }

    pub async fn set_todo_tags(&self, todo_id: i64, tag_ids: &[i64]) {
        self.remove_todo_tags(todo_id).await;
        for tag_id in tag_ids {
            self.add_todo_tag(todo_id, *tag_id).await;
        }
    }

    // ===== Execution record operations =====

    pub async fn get_execution_records(
        &self,
        todo_id: i64,
        limit: i64,
        offset: i64,
    ) -> (Vec<ExecutionRecord>, i64) {
        let total: i64 = execution_records::Entity::find()
            .filter(execution_records::Column::TodoId.eq(todo_id))
            .count(&self.conn)
            .await
            .unwrap_or(0) as i64;

        let limit_u = if limit < 0 { 0 } else { limit as u64 };
        let offset_u = if offset < 0 { 0 } else { offset as u64 };

        let records = execution_records::Entity::find()
            .filter(execution_records::Column::TodoId.eq(todo_id))
            .order_by_desc(execution_records::Column::StartedAt)
            .limit(limit_u)
            .offset(offset_u)
            .all(&self.conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| {
                let usage = m
                    .usage
                    .as_deref()
                    .and_then(|u| serde_json::from_str(u).ok());
                ExecutionRecord {
                    id: m.id,
                    todo_id: m.todo_id.unwrap_or(0),
                    status: m.status.unwrap_or_default(),
                    command: m.command.unwrap_or_default(),
                    stdout: m.stdout.unwrap_or_default(),
                    stderr: m.stderr.unwrap_or_default(),
                    logs: m.logs.unwrap_or_default(),
                    result: m.result,
                    started_at: m.started_at.unwrap_or_default(),
                    finished_at: m.finished_at,
                    usage,
                    executor: m.executor,
                    model: m.model,
                    trigger_type: m.trigger_type.unwrap_or_else(|| "manual".to_string()),
                }
            })
            .collect();

        (records, total)
    }

    pub async fn create_execution_record(
        &self,
        todo_id: i64,
        command: &str,
        executor: &str,
        trigger_type: &str,
    ) -> i64 {
        let now = now_utc();
        let am = execution_records::ActiveModel {
            todo_id: ActiveValue::Set(Some(todo_id)),
            command: ActiveValue::Set(Some(command.to_string())),
            executor: ActiveValue::Set(Some(executor.to_string())),
            trigger_type: ActiveValue::Set(Some(trigger_type.to_string())),
            status: ActiveValue::Set(Some("running".to_string())),
            started_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am
            .insert(&self.conn)
            .await
            .expect("insert execution record failed");
        inserted.id
    }

    pub async fn update_execution_record(&self, id: i64, status: &str, logs: &str, result: &str) {
        let now = now_utc();
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            logs: ActiveValue::Set(Some(logs.to_string())),
            result: ActiveValue::Set(Some(result.to_string())),
            finished_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_execution_record_with_usage(
        &self,
        id: i64,
        status: &str,
        logs: &str,
        result: &str,
        usage: &ExecutionUsage,
    ) {
        let now = now_utc();
        let usage_json = serde_json::to_string(usage).unwrap_or_default();
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            status: ActiveValue::Set(Some(status.to_string())),
            logs: ActiveValue::Set(Some(logs.to_string())),
            result: ActiveValue::Set(Some(result.to_string())),
            usage: ActiveValue::Set(Some(usage_json)),
            finished_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_execution_record_with_model(&self, id: i64, model: &str) {
        let am = execution_records::ActiveModel {
            id: ActiveValue::Unchanged(id),
            model: ActiveValue::Set(Some(model.to_string())),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn update_todo_status(&self, todo_id: i64, status: &str) {
        let now = now_utc();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(todo_id),
            status: ActiveValue::Set(Some(status.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    /// Mark a todo as running and associate it with a task_id.
    pub async fn start_todo_execution(&self, todo_id: i64, task_id: &str) {
        let now = now_utc();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(todo_id),
            status: ActiveValue::Set(Some("running".to_string())),
            task_id: ActiveValue::Set(Some(task_id.to_string())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    /// Mark a todo as completed or failed and clear its task_id.
    pub async fn finish_todo_execution(&self, todo_id: i64, success: bool) {
        let status = if success { "completed" } else { "failed" };
        let now = now_utc();
        let am = todos::ActiveModel {
            id: ActiveValue::Unchanged(todo_id),
            status: ActiveValue::Set(Some(status.to_string())),
            task_id: ActiveValue::Set(None),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let _ = am.update(&self.conn).await;
    }

    pub async fn force_update_todo_status_by_record(&self, record_id: i64, status: &str) {
        if let Ok(Some(record)) = execution_records::Entity::find_by_id(record_id)
            .one(&self.conn)
            .await
        {
            if let Some(todo_id) = record.todo_id {
                self.force_update_todo_status(todo_id, status).await;
            }
        }
    }

    pub async fn get_execution_summary(&self, todo_id: i64) -> ExecutionSummary {
        let records = execution_records::Entity::find()
            .filter(execution_records::Column::TodoId.eq(todo_id))
            .all(&self.conn)
            .await
            .unwrap_or_default();

        let mut total_executions = 0i64;
        let mut success_count = 0i64;
        let mut failed_count = 0i64;
        let mut running_count = 0i64;
        let mut total_input_tokens = 0u64;
        let mut total_output_tokens = 0u64;
        let mut total_cache_read_tokens = 0u64;
        let mut total_cache_creation_tokens = 0u64;
        let mut total_cost = 0.0f64;

        for r in records {
            total_executions += 1;
            match r.status.as_deref() {
                Some("success") => success_count += 1,
                Some("failed") => failed_count += 1,
                Some("running") => running_count += 1,
                _ => {}
            }
            if let Some(usage_str) = r.usage {
                if let Ok(usage) = serde_json::from_str::<ExecutionUsage>(&usage_str) {
                    total_input_tokens += usage.input_tokens;
                    total_output_tokens += usage.output_tokens;
                    total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
                    total_cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
                    if let Some(cost) = usage.total_cost_usd {
                        total_cost += cost;
                    }
                }
            }
        }

        ExecutionSummary {
            todo_id,
            total_executions,
            success_count,
            failed_count,
            running_count,
            total_input_tokens,
            total_output_tokens,
            total_cache_read_tokens,
            total_cache_creation_tokens,
            total_cost_usd: if total_cost > 0.0 { Some(total_cost) } else { None },
        }
    }
}

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
        db.update_todo(id, "Updated", "Desc", "in_progress").await;
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

        let model = todos::Entity::find_by_id(id).one(&db.conn).await.unwrap().unwrap();
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
        let record_id = db.create_execution_record(todo_id, "echo hi", "claudecode", "manual").await;
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
        let record_id = db.create_execution_record(todo_id, "echo hi", "claudecode", "manual").await;

        let before = truncate_seconds(Utc::now());
        db.update_execution_record(record_id, "success", "[]", "done").await;
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
