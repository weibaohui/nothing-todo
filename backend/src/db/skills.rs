use sea_orm::{ConnectionTrait, Statement, DbBackend, Value};

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
        let backend = self.conn.get_database_backend();

        let (sql, params): (String, Vec<Value>) = match (skill_name, executor) {
            (Some(name), Some(ex)) => (
                "SELECT si.id, si.skill_name, si.executor, si.todo_id, t.title as todo_title, \
                 si.status, si.duration_ms, si.invoked_at \
                 FROM skill_invocations si \
                 LEFT JOIN todos t ON t.id = si.todo_id \
                 WHERE si.skill_name = $1 AND si.executor = $2 \
                 ORDER BY si.invoked_at DESC \
                 LIMIT $3 OFFSET $4".to_string(),
                vec![name.into(), ex.into(), limit.into(), offset.into()],
            ),
            (Some(name), None) => (
                "SELECT si.id, si.skill_name, si.executor, si.todo_id, t.title as todo_title, \
                 si.status, si.duration_ms, si.invoked_at \
                 FROM skill_invocations si \
                 LEFT JOIN todos t ON t.id = si.todo_id \
                 WHERE si.skill_name = $1 \
                 ORDER BY si.invoked_at DESC \
                 LIMIT $2 OFFSET $3".to_string(),
                vec![name.into(), limit.into(), offset.into()],
            ),
            (None, Some(ex)) => (
                "SELECT si.id, si.skill_name, si.executor, si.todo_id, t.title as todo_title, \
                 si.status, si.duration_ms, si.invoked_at \
                 FROM skill_invocations si \
                 LEFT JOIN todos t ON t.id = si.todo_id \
                 WHERE si.executor = $1 \
                 ORDER BY si.invoked_at DESC \
                 LIMIT $2 OFFSET $3".to_string(),
                vec![ex.into(), limit.into(), offset.into()],
            ),
            (None, None) => (
                "SELECT si.id, si.skill_name, si.executor, si.todo_id, t.title as todo_title, \
                 si.status, si.duration_ms, si.invoked_at \
                 FROM skill_invocations si \
                 LEFT JOIN todos t ON t.id = si.todo_id \
                 ORDER BY si.invoked_at DESC \
                 LIMIT $1 OFFSET $2".to_string(),
                vec![limit.into(), offset.into()],
            ),
        };

        let statement = Statement::from_sql_and_values(backend, sql, params);
        let rows = self.conn.query_all(statement).await?;

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
        let backend = self.conn.get_database_backend();

        let (sql, params) = if let Some(d) = duration_ms {
            (
                "INSERT INTO skill_invocations (skill_name, executor, todo_id, status, duration_ms) \
                 VALUES ($1, $2, $3, $4, $5)".to_string(),
                vec![skill_name.into(), executor.into(), todo_id.into(), status.into(), d.into()],
            )
        } else {
            (
                "INSERT INTO skill_invocations (skill_name, executor, todo_id, status) \
                 VALUES ($1, $2, $3, $4)".to_string(),
                vec![skill_name.into(), executor.into(), todo_id.into(), status.into()],
            )
        };

        self.conn.execute(Statement::from_sql_and_values(backend, sql, params)).await?;

        let result = self.conn.query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT last_insert_rowid()".to_string(),
        )).await?;

        result
            .and_then(|r| r.try_get_by_index(0).ok())
            .flatten()
            .ok_or_else(|| sea_orm::DbErr::Query(sea_orm::RuntimeErr::Internal("Failed to get last_insert_rowid".to_string())))
    }
}
