use axum::extract::State;
use serde::Deserialize;

use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::ApiResponse;

/// Action 执行请求。
///
/// 前端传 action_type + action_key，后端查找或自动创建对应的 todo，
/// 然后用 prompt + params 执行。
#[derive(Debug, Deserialize)]
pub struct ExecuteActionRequest {
    /// 动作类型（如 "title_optimize"、"prompt_optimize"）
    pub action_type: String,
    /// 动作键值（如 "default"、"aggressive"）
    pub action_key: String,
    /// Prompt 模板（支持 {{key}} 占位符）
    pub prompt: String,
    /// 模板参数
    pub params: std::collections::HashMap<String, String>,
    /// 工作空间 ID（可选，不传则使用默认工作空间）
    pub workspace_id: Option<i64>,
}

/// Action 执行结果
#[derive(Debug, serde::Serialize)]
pub struct ExecuteActionResult {
    pub task_id: String,
    pub record_id: i64,
    pub todo_id: i64,
    /// todo 是否是本次自动创建的
    pub todo_created: bool,
}

/// POST /api/actions/execute
///
/// 根据 action_type + action_key 查找 todo，如果不存在则自动创建。
/// 然后用 prompt + params 执行该 todo。
pub async fn execute_action(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ExecuteActionRequest>,
) -> Result<ApiResponse<ExecuteActionResult>, AppError> {
    // 1. 查找或创建 todo
    let (todo_id, todo_created) = find_or_create_todo(&state, &req).await?;

    // 2. 构造 message：将 prompt 中的占位符替换为 params 中的值
    let message = replace_placeholders(&req.prompt, &req.params);

    // 3. 执行 todo
    let result = crate::handlers::execution::start_todo_execution(
        crate::executor_service::RunTodoExecutionRequest {
            db: state.db.clone(),
            executor_registry: state.executor_registry.clone(),
            tx: state.tx.clone(),
            task_manager: state.task_manager.clone(),
            config: state.config.clone(),
            todo_id,
            message,
            req_executor: None,
            trigger_type: "action".to_string(),
            params: Some(req.params.clone()),
            resume_session_id: None,
            resume_message: None,
            source_todo_id: None,
            source_todo_title: None,
            loop_step_execution_id: None,
            step_id: None,
            feishu_bot_id: None,
            feishu_receive_id: None,
            workspace_path: None,
            workspace_id: None,
        },
    )
    .await?;

    let record_id = result
        .record_id
        .ok_or_else(|| AppError::Internal("执行启动失败：未获取到执行记录 ID".to_string()))?;

    Ok(ApiResponse::ok(ExecuteActionResult {
        task_id: result.task_id,
        record_id,
        todo_id,
        todo_created,
    }))
}

/// 查找或创建 action 模板 todo。
///
/// 返回 (todo_id, todo_created)。
async fn find_or_create_todo(
    state: &AppState,
    req: &ExecuteActionRequest,
) -> Result<(i64, bool), AppError> {
    // 1. 尝试查找已有的 todo
    let todos = state
        .db
        .get_todos_by_workspace_id(None)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if let Some(todo) = todos.iter().find(|t| {
        t.action_type.as_deref() == Some(&req.action_type)
            && t.action_key.as_deref() == Some(&req.action_key)
    }) {
        return Ok((todo.id, false));
    }

    // 2. 未找到，自动创建
    let workspace_id = req.workspace_id.unwrap_or(1); // 默认使用工作空间 1
    let dir = state
        .db
        .get_project_directory_by_id(workspace_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::BadRequest(format!("工作空间 {} 不存在", workspace_id)))?;

    let title = format!("Action: {}/{}", req.action_type, req.action_key);

    let todo_id = state
        .db
        .create_todo_with_extras(
            &title,
            &req.prompt,
            None,    // executor: 使用默认
            None,    // acceptance_criteria
            false,   // webhook_enabled
            workspace_id,
            &dir.path,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 更新 action_type 和 action_key
    state
        .db
        .update_todo_full(crate::db::TodoUpdate {
            id: todo_id,
            title: &title,
            prompt: &req.prompt,
            status: crate::models::TodoStatus::Pending,
            executor: None,
            scheduler_enabled: None,
            scheduler_config: None,
            scheduler_timezone: None,
            workspace_id: None,
            webhook_enabled: None,
            acceptance_criteria: None,
            auto_review_enabled: None,
            action_type: Some(&req.action_type),
            action_key: Some(&req.action_key),
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((todo_id, true))
}

/// 将 prompt 模板中的占位符替换为 params 中的值。
fn replace_placeholders(
    prompt: &str,
    params: &std::collections::HashMap<String, String>,
) -> String {
    let mut result = prompt.to_string();
    for (key, value) in params {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_placeholders() {
        let prompt = "优化标题：{{title}}，参考：{{prompt}}";
        let mut params = std::collections::HashMap::new();
        params.insert("title".to_string(), "fix bug".to_string());
        params.insert("prompt".to_string(), "帮我修复登录超时".to_string());

        let result = replace_placeholders(prompt, &params);
        assert_eq!(result, "优化标题：fix bug，参考：帮我修复登录超时");
    }

    #[test]
    fn test_replace_placeholders_no_match() {
        let prompt = "优化标题：{{title}}";
        let params = std::collections::HashMap::new();

        let result = replace_placeholders(prompt, &params);
        assert_eq!(result, "优化标题：{{title}}");
    }
}
