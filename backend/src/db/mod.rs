use rusqlite::{params, Connection, Result};
use parking_lot::Mutex;
use std::sync::Arc;
use chrono::Utc;
use std::str::FromStr;

use crate::models::{Todo, Tag, ExecutionRecord, ExecutionUsage, ExecutionSummary};

fn compute_next_run(cron_expr: &str) -> Option<String> {
    cron::Schedule::from_str(cron_expr).ok().and_then(|schedule| {
        schedule.upcoming(Utc).next().map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
    })
}

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS todos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                prompt TEXT DEFAULT '',
                status TEXT DEFAULT 'pending',
                created_at TEXT,
                updated_at TEXT,
                deleted_at TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT DEFAULT '#1890ff',
                created_at TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS todo_tags (
                todo_id INTEGER,
                tag_id INTEGER,
                PRIMARY KEY (todo_id, tag_id),
                FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Add new columns to todos table (for existing databases)
        conn.execute(
            "ALTER TABLE todos ADD COLUMN executor TEXT DEFAULT 'claudecode'",
            [],
        ).ok();

        conn.execute(
            "ALTER TABLE todos ADD COLUMN scheduler_enabled INTEGER DEFAULT 0",
            [],
        ).ok();

        conn.execute(
            "ALTER TABLE todos ADD COLUMN scheduler_config TEXT",
            [],
        ).ok();

conn.execute(
            "ALTER TABLE todos ADD COLUMN model TEXT",
            [],
        ).ok();

        conn.execute(
            "ALTER TABLE todos ADD COLUMN task_id TEXT",
            [],
        ).ok();

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS set_todos_created_at_utc AFTER INSERT ON todos
             WHEN new.created_at IS NULL OR new.created_at = ''
             BEGIN
                 UPDATE todos SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc') WHERE rowid = new.rowid;
             END",
            [],
        ).ok();

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS set_todos_updated_at_utc BEFORE UPDATE ON todos
             WHEN new.updated_at IS NULL OR new.updated_at = ''
             BEGIN
                 SELECT raise(IGNORE);
             END",
            [],
        ).ok();

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS set_tags_created_at_utc AFTER INSERT ON tags
             WHEN new.created_at IS NULL OR new.created_at = ''
             BEGIN
                 UPDATE tags SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc') WHERE rowid = new.rowid;
             END",
            [],
        ).ok();

        // Add usage column if not exists (for existing databases)
        conn.execute(
            "ALTER TABLE execution_records ADD COLUMN usage TEXT",
            [],
        ).ok(); // Ignore error if column already exists

        // Add executor column if not exists
        conn.execute(
            "ALTER TABLE execution_records ADD COLUMN executor TEXT",
            [],
        ).ok(); // Ignore error if column already exists

        // Add model column if not exists
        conn.execute(
            "ALTER TABLE execution_records ADD COLUMN model TEXT",
            [],
        ).ok(); // Ignore error if column already exists

        conn.execute(
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
            [],
        )?;

        conn.execute(
            "ALTER TABLE execution_records ADD COLUMN trigger_type TEXT DEFAULT 'manual'",
            [],
        ).ok();

        // Rename description column to prompt (for existing databases)
        conn.execute(
            "ALTER TABLE todos RENAME COLUMN description TO prompt",
            [],
        ).ok();

        Ok(())
    }

    fn row_to_todo(&self, row: &rusqlite::Row) -> Result<Todo> {
        let scheduler_enabled: bool = row.get::<_, i64>(7)? != 0;
        let scheduler_config: Option<String> = row.get(8)?;
        let scheduler_next_run_at = if scheduler_enabled {
            scheduler_config.as_ref().and_then(|c| compute_next_run(c))
        } else {
            None
        };
        Ok(Todo {
            id: row.get(0)?,
            title: row.get(1)?,
            prompt: row.get(2)?,
            status: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            tag_ids: vec![],
            executor: row.get(6)?,
            scheduler_enabled,
            scheduler_config,
            scheduler_next_run_at,
            task_id: row.get(9)?,
        })
    }

    // Todo operations
    pub fn get_todos(&self) -> Vec<Todo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, prompt, status, created_at, updated_at, executor, scheduler_enabled, scheduler_config, task_id FROM todos WHERE deleted_at IS NULL ORDER BY updated_at DESC"
        ).unwrap();
        let todos: Vec<Todo> = stmt.query_map([], |row| {
            self.row_to_todo(row)
        }).unwrap().filter_map(|r| r.ok()).collect();

        drop(stmt);

        // Fetch tag_ids for each todo
        let mut tag_stmt = conn.prepare(
            "SELECT tag_id FROM todo_tags WHERE todo_id = ?1"
        ).unwrap();

        let mut result = Vec::new();
        for mut todo in todos {
            let tag_ids: Vec<i64> = tag_stmt.query_map([todo.id], |row| {
                row.get(0)
            }).unwrap().filter_map(|r| r.ok()).collect();
            todo.tag_ids = tag_ids;
            result.push(todo);
        }

        result
    }

    pub fn create_todo(&self, title: &str, prompt: &str) -> i64 {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        conn.execute(
            "INSERT INTO todos (title, prompt, created_at, updated_at, executor, status) VALUES (?1, ?2, ?3, ?3, 'claudecode', 'pending')",
            params![title, prompt, now],
        ).unwrap();
        conn.last_insert_rowid()
    }

    pub fn update_todo(&self, id: i64, title: &str, prompt: &str, status: &str) {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        conn.execute(
            "UPDATE todos SET title = ?1, prompt = ?2, status = ?3, updated_at = ?4 WHERE id = ?5",
            params![title, prompt, status, now, id],
        ).unwrap();
    }

    pub fn update_todo_full(&self, id: i64, title: &str, prompt: &str, status: &str, executor: Option<&str>, scheduler_enabled: Option<bool>, scheduler_config: Option<&str>) {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        if let Some(exec) = executor {
            conn.execute(
                "UPDATE todos SET executor = ?1 WHERE id = ?2",
                params![exec, id],
            ).unwrap();
        }

        if let Some(enabled) = scheduler_enabled {
            let val = if enabled { 1 } else { 0 };
            conn.execute(
                "UPDATE todos SET scheduler_enabled = ?1 WHERE id = ?2",
                params![val, id],
            ).unwrap();
        }

        if scheduler_config.is_some() {
            conn.execute(
                "UPDATE todos SET scheduler_config = ?1 WHERE id = ?2",
                params![scheduler_config, id],
            ).unwrap();
        }

        conn.execute(
            "UPDATE todos SET title = ?1, prompt = ?2, status = ?3, updated_at = ?4 WHERE id = ?5",
            params![title, prompt, status, now, id],
        ).unwrap();
    }

    pub fn update_todo_executor(&self, id: i64, executor: &str) {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE todos SET executor = ?1 WHERE id = ?2",
            params![executor, id],
        ).unwrap();
    }

    pub fn update_todo_task_id(&self, id: i64, task_id: Option<&str>) {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE todos SET task_id = ?1 WHERE id = ?2",
            params![task_id, id],
        ).unwrap();
    }

    pub fn update_todo_scheduler(&self, id: i64, enabled: bool, config: Option<&str>) {
        let conn = self.conn.lock();
        let enabled_val = if enabled { 1 } else { 0 };
        conn.execute(
            "UPDATE todos SET scheduler_enabled = ?1, scheduler_config = ?2 WHERE id = ?3",
            params![enabled_val, config, id],
        ).unwrap();
    }

    pub fn force_update_todo_status(&self, id: i64, status: &str) {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        conn.execute(
            "UPDATE todos SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, id],
        ).unwrap();
    }

    pub fn delete_todo(&self, id: i64) {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        conn.execute(
            "UPDATE todos SET deleted_at = ?1 WHERE id = ?2",
            params![now, id],
        ).unwrap();
    }

    pub fn get_todo(&self, id: i64) -> Option<Todo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, prompt, status, created_at, updated_at, executor, scheduler_enabled, scheduler_config, task_id FROM todos WHERE id = ?1 AND deleted_at IS NULL"
        ).unwrap();
        let mut todo: Option<Todo> = stmt.query_row(params![id], |row| {
            self.row_to_todo(row)
        }).ok();

        if let Some(ref mut t) = todo {
            let mut tag_stmt = conn.prepare("SELECT tag_id FROM todo_tags WHERE todo_id = ?1").unwrap();
            let tag_ids: Vec<i64> = tag_stmt.query_map([id], |row| row.get(0)).unwrap().filter_map(|r| r.ok()).collect();
            t.tag_ids = tag_ids;
        }

        todo
    }

    pub fn get_scheduler_todos(&self) -> Vec<Todo> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, prompt, status, created_at, updated_at, executor, scheduler_enabled, scheduler_config, task_id FROM todos WHERE deleted_at IS NULL AND scheduler_config IS NOT NULL"
        ).unwrap();
        let todos: Vec<Todo> = stmt.query_map([], |row| {
            self.row_to_todo(row)
        }).unwrap().filter_map(|r| r.ok()).collect();

        drop(stmt);

        let mut tag_stmt = conn.prepare(
            "SELECT tag_id FROM todo_tags WHERE todo_id = ?1"
        ).unwrap();

        let mut result = Vec::new();
        for mut todo in todos {
            let tag_ids: Vec<i64> = tag_stmt.query_map([todo.id], |row| {
                row.get(0)
            }).unwrap().filter_map(|r| r.ok()).collect();
            todo.tag_ids = tag_ids;
            result.push(todo);
        }

        result
    }

    // Tag operations
    pub fn get_tags(&self) -> Vec<Tag> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, name, color, created_at FROM tags ORDER BY name").unwrap();
        stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn create_tag(&self, name: &str, color: &str) -> i64 {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        conn.execute(
            "INSERT INTO tags (name, color, created_at) VALUES (?1, ?2, ?3)",
            params![name, color, now],
        ).unwrap();
        conn.last_insert_rowid()
    }

    pub fn delete_tag(&self, id: i64) {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM tags WHERE id = ?1", params![id]).unwrap();
    }

    pub fn add_todo_tag(&self, todo_id: i64, tag_id: i64) {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO todo_tags (todo_id, tag_id) VALUES (?1, ?2)",
            params![todo_id, tag_id],
        ).unwrap();
    }

    pub fn remove_todo_tags(&self, todo_id: i64) {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM todo_tags WHERE todo_id = ?1", params![todo_id]).unwrap();
    }

    pub fn set_todo_tags(&self, todo_id: i64, tag_ids: &[i64]) {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM todo_tags WHERE todo_id = ?1", params![todo_id]).unwrap();
        for tag_id in tag_ids {
            conn.execute(
                "INSERT OR IGNORE INTO todo_tags (todo_id, tag_id) VALUES (?1, ?2)",
                params![todo_id, tag_id],
            ).unwrap();
        }
    }

    // Execution record operations
    pub fn get_execution_records(&self, todo_id: i64, limit: i64, offset: i64) -> (Vec<ExecutionRecord>, i64) {
        let conn = self.conn.lock();

        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM execution_records WHERE todo_id = ?1",
            params![todo_id],
            |row| row.get(0),
        ).unwrap_or(0);

        let mut stmt = conn.prepare(
            "SELECT id, todo_id, status, command, stdout, stderr, logs, result, started_at, finished_at, usage, executor, model, trigger_type
             FROM execution_records WHERE todo_id = ?1 ORDER BY started_at DESC LIMIT ?2 OFFSET ?3"
        ).unwrap();
        let records = stmt.query_map(params![todo_id, limit, offset], |row| {
            let usage_str: Option<String> = row.get(10)?;
            let usage = usage_str.and_then(|u| serde_json::from_str(&u).ok());
            let executor: Option<String> = row.get(11)?;
            let model: Option<String> = row.get(12)?;
            let trigger_type: Option<String> = row.get(13)?;
            Ok(ExecutionRecord {
                id: row.get(0)?,
                todo_id: row.get(1)?,
                status: row.get(2)?,
                command: row.get(3)?,
                stdout: row.get(4)?,
                stderr: row.get(5)?,
                logs: row.get(6)?,
                result: row.get(7)?,
                started_at: row.get(8)?,
                finished_at: row.get(9)?,
                usage,
                executor,
                model,
                trigger_type: trigger_type.unwrap_or_else(|| "manual".to_string()),
            })
        }).unwrap().filter_map(|r| r.ok()).collect();

        (records, total)
    }

    pub fn create_execution_record(&self, todo_id: i64, command: &str, executor: &str, trigger_type: &str) -> i64 {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO execution_records (todo_id, command, executor, trigger_type, status, started_at) VALUES (?1, ?2, ?3, ?4, 'running', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![todo_id, command, executor, trigger_type],
        ).unwrap();
        conn.last_insert_rowid()
    }

    pub fn update_execution_record(&self, id: i64, status: &str, logs: &str, result: &str) {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE execution_records SET status = ?1, logs = ?2, result = ?3, finished_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?4",
            params![status, logs, result, id],
        ).unwrap();
    }

    pub fn update_execution_record_with_usage(&self, id: i64, status: &str, logs: &str, result: &str, usage: &ExecutionUsage) {
        let conn = self.conn.lock();
        let usage_json = serde_json::to_string(usage).unwrap_or_default();
        conn.execute(
            "UPDATE execution_records SET status = ?1, logs = ?2, result = ?3, usage = ?4, finished_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?5",
            params![status, logs, result, usage_json, id],
        ).unwrap();
    }

    pub fn update_execution_record_with_model(&self, id: i64, model: &str) {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE execution_records SET model = ?1 WHERE id = ?2",
            params![model, id],
        ).unwrap();
    }

    pub fn update_todo_status(&self, todo_id: i64, status: &str) {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE todos SET status = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?2",
            params![status, todo_id],
        ).unwrap();
    }

    /// Mark a todo as running and associate it with a task_id.
    pub fn start_todo_execution(&self, todo_id: i64, task_id: &str) {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE todos SET status = 'running', task_id = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?2",
            params![task_id, todo_id],
        ).unwrap();
    }

    /// Mark a todo as completed or failed and clear its task_id.
    pub fn finish_todo_execution(&self, todo_id: i64, success: bool) {
        let status = if success { "completed" } else { "failed" };
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE todos SET status = ?1, task_id = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?2",
            params![status, todo_id],
        ).unwrap();
    }

    pub fn force_update_todo_status_by_record(&self, record_id: i64, status: &str) {
        let conn = self.conn.lock();
        // First get the todo_id from record
        let mut stmt = conn.prepare("SELECT todo_id FROM execution_records WHERE id = ?1").unwrap();
        if let Ok(todo_id) = stmt.query_row(params![record_id], |row| row.get::<_, i64>(0)) {
            drop(stmt);
            let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            conn.execute(
                "UPDATE todos SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status, now, todo_id],
            ).unwrap();
        }
    }

    pub fn get_execution_summary(&self, todo_id: i64) -> ExecutionSummary {
        let records: Vec<(String, Option<String>)> = {
            let conn = self.conn.lock();
            let mut stmt = conn.prepare(
                "SELECT status, usage FROM execution_records WHERE todo_id = ?1"
            ).unwrap();
            stmt.query_map(params![todo_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            }).unwrap().filter_map(|r| r.ok()).collect()
        };

        let mut total_executions = 0i64;
        let mut success_count = 0i64;
        let mut failed_count = 0i64;
        let mut running_count = 0i64;
        let mut total_input_tokens = 0u64;
        let mut total_output_tokens = 0u64;
        let mut total_cache_read_tokens = 0u64;
        let mut total_cache_creation_tokens = 0u64;
        let mut total_cost = 0.0f64;

        for record in records {
            total_executions += 1;
            match record.0.as_str() {
                "success" => success_count += 1,
                "failed" => failed_count += 1,
                "running" => running_count += 1,
                _ => {}
            }

            if let Some(usage_str) = record.1 {
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

    fn setup_db() -> Database {
        Database::new(":memory:").unwrap()
    }

    fn parse_utc(ts: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc)
    }

    fn truncate_seconds(dt: DateTime<Utc>) -> DateTime<Utc> {
        dt.with_nanosecond(0).unwrap()
    }

    #[test]
    fn test_todo_created_at_is_utc() {
        let db = setup_db();
        let before = truncate_seconds(Utc::now());
        let id = db.create_todo("Test", "Desc");
        let after = truncate_seconds(Utc::now());

        let todo = db.get_todo(id).unwrap();
        let created = parse_utc(&todo.created_at);

        assert!(created >= before, "created_at should not be before test start");
        assert!(created <= after, "created_at should not be after test end");
        assert!(todo.created_at.ends_with('Z'), "UTC timestamp must end with Z");
    }

    #[test]
    fn test_todo_updated_at_changes_on_update() {
        let db = setup_db();
        let id = db.create_todo("Test", "Desc");
        let original = db.get_todo(id).unwrap().updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1100));
        db.update_todo(id, "Updated", "Desc", "in_progress");
        let updated = db.get_todo(id).unwrap().updated_at;

        assert_ne!(original, updated, "updated_at should change after update");
        assert!(updated.ends_with('Z'));
    }

    #[test]
    fn test_todo_deleted_at_is_utc() {
        let db = setup_db();
        let id = db.create_todo("Test", "Desc");
        let before = truncate_seconds(Utc::now());
        db.delete_todo(id);
        let after = truncate_seconds(Utc::now());

        let conn = db.conn.lock();
        let deleted_at: String = conn
            .query_row("SELECT deleted_at FROM todos WHERE id = ?1", [id], |row| row.get(0))
            .unwrap();

        let dt = parse_utc(&deleted_at);
        assert!(dt >= before);
        assert!(dt <= after);
        assert!(deleted_at.ends_with('Z'));
    }

    #[test]
    fn test_tag_created_at_is_utc() {
        let db = setup_db();
        let before = truncate_seconds(Utc::now());
        let id = db.create_tag("urgent", "#ff0000");
        let after = truncate_seconds(Utc::now());

        let tag = db.get_tags().into_iter().find(|t| t.id == id).unwrap();
        let created = parse_utc(&tag.created_at);

        assert!(created >= before);
        assert!(created <= after);
        assert!(tag.created_at.ends_with('Z'));
    }

    #[test]
    fn test_execution_record_started_at_is_utc() {
        let db = setup_db();
        let todo_id = db.create_todo("Test", "Desc");
        let before = truncate_seconds(Utc::now());
        let record_id = db.create_execution_record(todo_id, "echo hi", "claudecode", "manual");
        let after = truncate_seconds(Utc::now());

        let records = db.get_execution_records(todo_id);
        let record = records.into_iter().find(|r| r.id == record_id).unwrap();
        let started = parse_utc(&record.started_at);

        assert!(started >= before);
        assert!(started <= after);
        assert!(record.started_at.ends_with('Z'));
    }

    #[test]
    fn test_execution_record_finished_at_is_utc() {
        let db = setup_db();
        let todo_id = db.create_todo("Test", "Desc");
        let record_id = db.create_execution_record(todo_id, "echo hi", "claudecode", "manual");

        let before = truncate_seconds(Utc::now());
        db.update_execution_record(record_id, "success", "[]", "done");
        let after = truncate_seconds(Utc::now());

        let records = db.get_execution_records(todo_id);
        let record = records.into_iter().find(|r| r.id == record_id).unwrap();
        let finished_at = record.finished_at.unwrap();
        let finished = parse_utc(&finished_at);

        assert!(finished >= before);
        assert!(finished <= after);
        assert!(finished_at.ends_with('Z'));
    }

    #[test]
    fn test_sqlite_default_timestamp_is_utc() {
        let db = setup_db();
        let conn = db.conn.lock();
        let ts: String = conn
            .query_row(
                "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(ts.ends_with('Z'), "SQLite default timestamp must end with Z");
        assert!(DateTime::parse_from_rfc3339(&ts).is_ok());
    }
}
