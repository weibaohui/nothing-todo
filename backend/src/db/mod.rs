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
                description TEXT DEFAULT '',
                status TEXT DEFAULT 'pending',
                created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                deleted_at TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT DEFAULT '#1890ff',
                created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
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
            "ALTER TABLE todos ADD COLUMN task_id TEXT",
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
                FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
            )",
            [],
        )?;

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
            description: row.get(2)?,
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
            "SELECT id, title, description, status, created_at, updated_at, executor, scheduler_enabled, scheduler_config, task_id FROM todos WHERE deleted_at IS NULL ORDER BY created_at DESC"
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

    pub fn create_todo(&self, title: &str, description: &str) -> i64 {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        conn.execute(
            "INSERT INTO todos (title, description, created_at, updated_at, executor) VALUES (?1, ?2, ?3, ?3, 'claudecode')",
            params![title, description, now],
        ).unwrap();
        conn.last_insert_rowid()
    }

    pub fn update_todo(&self, id: i64, title: &str, description: &str, status: &str) {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        conn.execute(
            "UPDATE todos SET title = ?1, description = ?2, status = ?3, updated_at = ?4 WHERE id = ?5",
            params![title, description, status, now, id],
        ).unwrap();
    }

    pub fn update_todo_full(&self, id: i64, title: &str, description: &str, status: &str, executor: Option<&str>, scheduler_enabled: Option<bool>, scheduler_config: Option<&str>) {
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
            "UPDATE todos SET title = ?1, description = ?2, status = ?3, updated_at = ?4 WHERE id = ?5",
            params![title, description, status, now, id],
        ).unwrap();
    }

    pub fn update_todo_executor(&self, id: i64, executor: &str) {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE todos SET executor = ?1 WHERE id = ?2",
            params![executor, id],
        ).unwrap();
    }

    pub fn update_todo_scheduler(&self, id: i64, enabled: bool, config: Option<&str>, task_id: Option<&str>) {
        let conn = self.conn.lock();
        let enabled_val = if enabled { 1 } else { 0 };
        conn.execute(
            "UPDATE todos SET scheduler_enabled = ?1, scheduler_config = ?2, task_id = ?3 WHERE id = ?4",
            params![enabled_val, config, task_id, id],
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
            "SELECT id, title, description, status, created_at, updated_at, executor, scheduler_enabled, scheduler_config, task_id FROM todos WHERE id = ?1 AND deleted_at IS NULL"
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
            "SELECT id, title, description, status, created_at, updated_at, executor, scheduler_enabled, scheduler_config, task_id FROM todos WHERE deleted_at IS NULL AND scheduler_config IS NOT NULL"
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
    pub fn get_execution_records(&self, todo_id: i64) -> Vec<ExecutionRecord> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, todo_id, status, command, stdout, stderr, logs, result, started_at, finished_at, usage, executor, model
             FROM execution_records WHERE todo_id = ?1 ORDER BY started_at DESC"
        ).unwrap();
        stmt.query_map(params![todo_id], |row| {
            let usage_str: Option<String> = row.get(10)?;
            let usage = usage_str.and_then(|u| serde_json::from_str(&u).ok());
            let executor: Option<String> = row.get(11)?;
            let model: Option<String> = row.get(12)?;
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
            })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn create_execution_record(&self, todo_id: i64, command: &str, executor: &str) -> i64 {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO execution_records (todo_id, command, executor, status, started_at) VALUES (?1, ?2, ?3, 'running', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![todo_id, command, executor],
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
