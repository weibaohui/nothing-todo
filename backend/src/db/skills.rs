use sea_orm::{ConnectionTrait, Statement, DbBackend};

use crate::db::Database;
use crate::handlers::skills::SkillInvocation;

impl Database {
    pub async fn get_skill_invocations(
        &self,
        offset: i64,
        limit: i64,
        skill_name: Option<&str>,
        executor: Option<&str>,
    ) -> Result<Vec<SkillInvocation>, sea_orm::DbErr> {
        let mut where_clauses = Vec::new();
        if let Some(name) = skill_name {
            where_clauses.push(format!("si.skill_name = '{}'", name.replace('\'', "''")));
        }
        if let Some(ex) = executor {
            where_clauses.push(format!("si.executor = '{}'", ex.replace('\'', "''")));
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let sql = format!(
            "SELECT si.id, si.skill_name, si.executor, si.todo_id, t.title as todo_title, \
             si.status, si.duration_ms, si.invoked_at \
             FROM skill_invocations si \
             LEFT JOIN todos t ON t.id = si.todo_id \
             {where_sql} \
             ORDER BY si.invoked_at DESC \
             LIMIT {limit} OFFSET {offset}"
        );

        let rows = self.conn.query_all(Statement::from_string(DbBackend::Sqlite, sql)).await?;
        let mut invocations = Vec::new();
        for row in rows {
            let id: i64 = row.try_get_by_index(0)?;
            let skill_name: String = row.try_get_by_index(1)?;
            let executor: String = row.try_get_by_index(2)?;
            let todo_id: i64 = row.try_get_by_index(3).unwrap_or(0);
            let todo_title: Option<String> = row.try_get_by_index(4).ok();
            let status: String = row.try_get_by_index(5)?;
            let duration_ms: Option<i64> = row.try_get_by_index(6).ok();
            let invoked_at: String = row.try_get_by_index(7)?;

            invocations.push(SkillInvocation {
                id,
                skill_name,
                executor,
                todo_id,
                todo_title,
                invoked_at,
                status,
                duration_ms,
            });
        }
        Ok(invocations)
    }

    pub async fn record_skill_invocation(
        &self,
        skill_name: &str,
        executor: &str,
        todo_id: i64,
        status: &str,
        duration_ms: Option<i64>,
    ) -> Result<i64, sea_orm::DbErr> {
        let duration_sql = duration_ms
            .map(|d| d.to_string())
            .unwrap_or_else(|| "NULL".to_string());

        let sql = format!(
            "INSERT INTO skill_invocations (skill_name, executor, todo_id, status, duration_ms) \
             VALUES ('{}', '{}', {}, '{}', {duration_sql})",
            skill_name.replace('\'', "''"),
            executor.replace('\'', "''"),
            todo_id,
            status.replace('\'', "''"),
        );

        self.conn.execute(Statement::from_string(DbBackend::Sqlite, sql)).await?;

        let result = self.conn.query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT last_insert_rowid()".to_string(),
        )).await?;

        Ok(result.map(|r| r.try_get_by_index(0).unwrap_or(0)).unwrap_or(0))
    }
}
