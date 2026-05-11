use axum::extract::{Path, State};
use std::path::PathBuf;
use std::time::Duration;

use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{ApiResponse, ExecutorConfig, ExecutorDetectResult, ExecutorTestResult, UpdateExecutorRequest};

pub async fn list_executors(State(state): State<AppState>) -> Result<ApiResponse<Vec<ExecutorConfig>>, AppError> {
    let executors = state.db.get_executors().await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(ApiResponse::ok(executors))
}

pub async fn update_executor(
    State(state): State<AppState>,
    Path(name): Path<String>,
    ApiJson(req): ApiJson<UpdateExecutorRequest>,
) -> Result<ApiResponse<ExecutorConfig>, AppError> {
    state.db.update_executor(
        &name,
        req.path.as_deref(),
        req.enabled,
        req.display_name.as_deref(),
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    // Re-read updated executor
    let ec = state.db.get_executor_by_name(&name).await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound)?;

    // Update registry based on enabled state
    if ec.enabled {
        state.executor_registry.register_by_name(&ec.name, &ec.path);
    } else {
        if let Some(et) = crate::adapters::parse_executor_type(&ec.name) {
            state.executor_registry.unregister(et);
        }
    }

    Ok(ApiResponse::ok(ec))
}

pub async fn detect_executor(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<ApiResponse<ExecutorDetectResult>, AppError> {
    let ec = state.db.get_executor_by_name(&name).await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound)?;

    let path = if ec.path.is_empty() { name.clone() } else { ec.path.clone() };
    let (found, resolved) = detect_binary(&path);

    Ok(ApiResponse::ok(ExecutorDetectResult {
        binary_found: found,
        path_resolved: resolved,
    }))
}

pub async fn test_executor(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<ApiResponse<ExecutorTestResult>, AppError> {
    let ec = state.db.get_executor_by_name(&name).await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound)?;

    let path = if ec.path.is_empty() { name.clone() } else { ec.path.clone() };
    let (found, _) = detect_binary(&path);

    if !found {
        return Ok(ApiResponse::ok(ExecutorTestResult {
            test_passed: false,
            output: None,
            error: Some(format!("Binary not found: {}", path)),
        }));
    }

    // Try running --version with a short timeout
    let output = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new(&path)
            .arg("--version")
            .output(),
    ).await;

    match output {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = if stdout.is_empty() { stderr } else { stdout };
            Ok(ApiResponse::ok(ExecutorTestResult {
                test_passed: out.status.success(),
                output: Some(combined.trim().to_string()),
                error: None,
            }))
        }
        Ok(Err(e)) => Ok(ApiResponse::ok(ExecutorTestResult {
            test_passed: false,
            output: None,
            error: Some(format!("Failed to execute: {}", e)),
        })),
        Err(_) => Ok(ApiResponse::ok(ExecutorTestResult {
            test_passed: false,
            output: None,
            error: Some("Execution timed out (10s)".to_string()),
        })),
    }
}

/// Check if a binary exists at the given path or in PATH.
fn detect_binary(path: &str) -> (bool, Option<String>) {
    let p = PathBuf::from(path);

    // If it looks like an absolute or relative path (contains separator)
    if p.is_absolute() || path.contains(std::path::MAIN_SEPARATOR) {
        if p.exists() {
            return (true, Some(p.to_string_lossy().to_string()));
        }
        return (false, None);
    }

    // Bare command name — look up in PATH using `which` equivalent
    match which::which(path) {
        Ok(resolved) => (true, Some(resolved.to_string_lossy().to_string())),
        Err(_) => (false, None),
    }
}
