use axum::{
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::{AppError, AppState};
use crate::models::ApiResponse;

// ─── Request / Response types ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,    // "active" | "completed"
    pub executor: Option<String>,  // filter by entrypoint
    pub project: Option<String>,   // filter by project path (partial match)
    pub search: Option<String>,    // search in first prompt
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub project_path: String,
    pub status: String,
    pub executor: String,
    pub model: String,
    pub git_branch: Option<String>,
    pub message_count: u32,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub first_prompt: Option<String>,
    pub created_at: Option<String>,
    pub last_active_at: Option<String>,
    pub file_size: u64,
    pub version: Option<String>,
    pub subagent_count: u32,
}

#[derive(Debug, Serialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionInfo>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub today_sessions: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub by_executor: HashMap<String, u64>,
    pub by_project: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionMessage {
    pub role: String,
    pub content_preview: String,
    pub model: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub timestamp: Option<String>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubAgentInfo {
    pub agent_type: String,
    pub description: String,
    pub message_count: u32,
}

#[derive(Debug, Serialize)]
pub struct SessionDetail {
    pub info: SessionInfo,
    pub messages: Vec<SessionMessage>,
    pub subagents: Vec<SubAgentInfo>,
}

// ─── Helpers ──────────────────────────────────────────────

fn claude_home() -> PathBuf {
    dirs::home_dir()
        .expect("no home directory")
        .join(".claude")
}

/// Decode project path from encoded directory name.
/// e.g. "-Users-weibh-projects-rust-aitodo" -> "/Users/weibh/projects/rust/aitodo"
fn decode_project_path(encoded: &str) -> String {
    let s = encoded.strip_prefix('-').unwrap_or(encoded);
    // The encoding replaces '/' with '-'. We need to restore them.
    // But path components like "/Users/weibh" also have separators.
    // Strategy: split on '-', try to reconstruct valid path with '/'.
    // Since home dir starts with /Users, we know the first segment is empty.
    format!("/{}", s.replace('-', "/"))
}

/// Truncate a string to at most `max_len` chars, appending "..." if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

/// Extract text content from a JSON value that may be a string or array of content blocks.
fn extract_text_content(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(blocks) => {
            let mut texts = Vec::new();
            for block in blocks {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    texts.push(text.to_string());
                }
            }
            texts.join("\n")
        }
        _ => String::new(),
    }
}

/// Parse a single JSONL line into session metadata contributions.
fn parse_line_metadata(
    line: &str,
) -> Option<(
    Option<String>,    // timestamp
    Option<String>,    // model
    Option<String>,    // git_branch
    Option<String>,    // version
    Option<String>,    // executor / entrypoint
    Option<String>,    // first prompt content
    Option<u64>,       // input_tokens
    Option<u64>,       // output_tokens
    String,            // role
)> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let msg_type = v.get("type")?.as_str()?;

    match msg_type {
        "user" => {
            let ts = v.get("timestamp").and_then(|t| t.as_str()).map(|s| s.to_string());
            let branch = v.get("gitBranch").and_then(|b| b.as_str()).map(|s| s.to_string());
            let ver = v.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
            let entry = v.get("entrypoint").and_then(|e| e.as_str()).map(|s| s.to_string());
            let content = v.get("message")
                .and_then(|m| m.get("content"))
                .map(|c| extract_text_content(c));
            Some((ts, None, branch, ver, entry, content, None, None, "user".to_string()))
        }
        "assistant" => {
            let ts = v.get("timestamp").and_then(|t| t.as_str()).map(|s| s.to_string());
            let msg = v.get("message")?;
            let model = msg.get("model").and_then(|m| m.as_str()).map(|s| s.to_string());
            let usage = msg.get("usage");
            let input_tokens = usage
                .and_then(|u| u.get("input_tokens"))
                .and_then(|t| t.as_u64());
            let output_tokens = usage
                .and_then(|u| u.get("output_tokens"))
                .and_then(|t| t.as_u64());
            Some((ts, model, None, None, None, None, input_tokens, output_tokens, "assistant".to_string()))
        }
        "queue-operation" => {
            if v.get("operation").and_then(|o| o.as_str()) == Some("enqueue") {
                let ts = v.get("timestamp").and_then(|t| t.as_str()).map(|s| s.to_string());
                let content = v.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                Some((ts, None, None, None, None, content, None, None, "queue".to_string()))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Collect all active session IDs from ~/.claude/sessions/
fn collect_active_sessions() -> std::collections::HashSet<String> {
    let sessions_dir = claude_home().join("sessions");
    let mut active = std::collections::HashSet::new();
    if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(sid) = v.get("sessionId").and_then(|s| s.as_str()) {
                        active.insert(sid.to_string());
                    }
                }
            }
        }
    }
    active
}

/// Scan all project directories and collect session JSONL metadata.
fn scan_all_sessions(
    active_set: &std::collections::HashSet<String>,
) -> Vec<SessionInfo> {
    let projects_dir = claude_home().join("projects");
    let mut sessions = Vec::new();

    if let Ok(project_entries) = std::fs::read_dir(&projects_dir) {
        for project_entry in project_entries.flatten() {
            let project_name = project_entry.file_name();
            let project_name_str = project_name.to_string_lossy().to_string();

            // Skip non-directory entries and known non-project dirs
            if !project_entry.path().is_dir() {
                continue;
            }
            if project_name_str.starts_with('.') || project_name_str == "memory" {
                continue;
            }

            let project_path = decode_project_path(&project_name_str);

            if let Ok(session_entries) = std::fs::read_dir(project_entry.path()) {
                for session_entry in session_entries.flatten() {
                    let path = session_entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                        continue;
                    }

                    let session_id = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();

                    let file_size = std::fs::metadata(&path)
                        .map(|m| m.len())
                        .unwrap_or(0);

                    // Parse JSONL for metadata — read up to 2MB to get initial metadata
                    let file_content = std::fs::read_to_string(&path).unwrap_or_default();

                    let mut first_timestamp: Option<String> = None;
                    let mut last_timestamp: Option<String> = None;
                    let mut model: Option<String> = None;
                    let mut git_branch: Option<String> = None;
                    let mut version: Option<String> = None;
                    let mut executor: Option<String> = None;
                    let mut first_prompt: Option<String> = None;
                    let mut message_count: u32 = 0;
                    let mut total_input_tokens: u64 = 0;
                    let mut total_output_tokens: u64 = 0;

                    for line in file_content.lines() {
                        if let Some((ts, mdl, branch, ver, entry, prompt, inp_tok, out_tok, role)) =
                            parse_line_metadata(line)
                        {
                            if first_timestamp.is_none() {
                                first_timestamp = ts.clone();
                            }
                            if ts.is_some() {
                                last_timestamp = ts;
                            }
                            if mdl.is_some() {
                                model = mdl;
                            }
                            if branch.is_some() {
                                git_branch = branch;
                            }
                            if ver.is_some() {
                                version = ver;
                            }
                            if entry.is_some() {
                                executor = entry;
                            }
                            if first_prompt.is_none() && prompt.is_some() {
                                first_prompt = prompt;
                            }
                            if role == "user" || role == "assistant" {
                                message_count += 1;
                            }
                            if let Some(inp) = inp_tok {
                                total_input_tokens += inp;
                            }
                            if let Some(out) = out_tok {
                                total_output_tokens += out;
                            }
                        }
                    }

                    let status = if active_set.contains(&session_id) {
                        "active"
                    } else {
                        "completed"
                    };

                    // Truncate first_prompt for display
                    let display_prompt = first_prompt.map(|p| {
                        if p.len() > 200 {
                            truncate_str(&p, 200)
                        } else {
                            p
                        }
                    });

                    sessions.push(SessionInfo {
                        session_id,
                        project_path: project_path.clone(),
                        status: status.to_string(),
                        executor: executor.unwrap_or_else(|| "unknown".to_string()),
                        model: model.unwrap_or_else(|| "-".to_string()),
                        git_branch,
                        message_count,
                        total_input_tokens,
                        total_output_tokens,
                        first_prompt: display_prompt,
                        created_at: first_timestamp,
                        last_active_at: last_timestamp,
                        file_size,
                        version,
                        subagent_count: 0, // will be filled later if needed
                    });
                }
            }
        }
    }

    // Sort by last_active_at descending
    sessions.sort_by(|a, b| {
        b.last_active_at
            .cmp(&a.last_active_at)
    });

    sessions
}

// ─── Handlers ─────────────────────────────────────────────

pub async fn list_sessions(
    State(_state): State<AppState>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<ApiResponse<SessionListResponse>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let sessions = tokio::task::spawn_blocking(move || {
        let active_set = collect_active_sessions();
        let mut sessions = scan_all_sessions(&active_set);

        // Apply filters
        if let Some(ref status) = query.status {
            sessions.retain(|s| &s.status == status);
        }
        if let Some(ref executor) = query.executor {
            sessions.retain(|s| s.executor == *executor);
        }
        if let Some(ref project) = query.project {
            sessions.retain(|s| s.project_path.contains(project));
        }
        if let Some(ref search) = query.search {
            let search_lower = search.to_lowercase();
            sessions.retain(|s| {
                s.first_prompt
                    .as_ref()
                    .map(|p| p.to_lowercase().contains(&search_lower))
                    .unwrap_or(false)
            });
        }

        let total = sessions.len() as u64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(sessions.len());
        let page_data = if start < sessions.len() {
            sessions[start..end].to_vec()
        } else {
            Vec::new()
        };

        SessionListResponse {
            sessions: page_data,
            total,
            page,
            page_size,
        }
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(ApiResponse::ok(sessions))
}

pub async fn get_session_stats(
    State(_state): State<AppState>,
) -> Result<ApiResponse<SessionStats>, AppError> {
    let stats = tokio::task::spawn_blocking(move || {
        let active_set = collect_active_sessions();
        let sessions = scan_all_sessions(&active_set);

        let now = chrono::Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();

        let mut by_executor: HashMap<String, u64> = HashMap::new();
        let mut by_project: HashMap<String, u64> = HashMap::new();
        let mut active_count = 0u64;
        let mut today_count = 0u64;
        let mut total_input = 0u64;
        let mut total_output = 0u64;

        for s in &sessions {
            *by_executor.entry(s.executor.clone()).or_insert(0) += 1;
            *by_project.entry(s.project_path.clone()).or_insert(0) += 1;
            total_input += s.total_input_tokens;
            total_output += s.total_output_tokens;
            if s.status == "active" {
                active_count += 1;
            }
            if let Some(ref created) = s.created_at {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created) {
                    if dt.naive_utc() >= today_start {
                        today_count += 1;
                    }
                }
            }
        }

        SessionStats {
            total_sessions: sessions.len() as u64,
            active_sessions: active_count,
            today_sessions: today_count,
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            by_executor,
            by_project,
        }
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(ApiResponse::ok(stats))
}

pub async fn get_session_detail(
    State(_state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<ApiResponse<SessionDetail>, AppError> {
    let detail = tokio::task::spawn_blocking(move || {
        let projects_dir = claude_home().join("projects");

        // Find the JSONL file for this session_id across all projects
        let mut jsonl_path: Option<PathBuf> = None;
        let mut project_path = String::new();

        if let Ok(project_entries) = std::fs::read_dir(&projects_dir) {
            for project_entry in project_entries.flatten() {
                let candidate = project_entry.path().join(format!("{}.jsonl", session_id));
                if candidate.exists() {
                    jsonl_path = Some(candidate);
                    let name = project_entry.file_name().to_string_lossy().to_string();
                    project_path = decode_project_path(&name);
                    break;
                }
            }
        }

        let jsonl_path = match jsonl_path {
            Some(p) => p,
            None => return None,
        };

        let active_set = collect_active_sessions();
        let file_content = std::fs::read_to_string(&jsonl_path).ok()?;
        let file_size = std::fs::metadata(&jsonl_path).map(|m| m.len()).unwrap_or(0);

        let mut first_timestamp: Option<String> = None;
        let mut last_timestamp: Option<String> = None;
        let mut model: Option<String> = None;
        let mut git_branch: Option<String> = None;
        let mut version: Option<String> = None;
        let mut executor: Option<String> = None;
        let mut first_prompt: Option<String> = None;
        let mut message_count: u32 = 0;
        let mut total_input_tokens: u64 = 0;
        let mut total_output_tokens: u64 = 0;
        let mut messages: Vec<SessionMessage> = Vec::new();

        for line in file_content.lines() {
            if let Some((ts, mdl, branch, ver, entry, prompt, inp_tok, out_tok, role)) =
                parse_line_metadata(line)
            {
                if first_timestamp.is_none() {
                    first_timestamp = ts.clone();
                }
                if ts.is_some() {
                    last_timestamp = ts.clone();
                }
                if mdl.is_some() {
                    model = mdl.clone();
                }
                if branch.is_some() {
                    git_branch = branch;
                }
                if ver.is_some() {
                    version = ver;
                }
                if entry.is_some() {
                    executor = entry;
                }
                if first_prompt.is_none() && prompt.is_some() {
                    first_prompt = prompt.clone();
                }
                if role == "user" || role == "assistant" {
                    message_count += 1;

                    let content_preview = match &prompt {
                        Some(p) => {
                            if p.len() > 500 {
                                truncate_str(&p, 500)
                            } else {
                                p.clone()
                            }
                        }
                        None => String::new(),
                    };

                    messages.push(SessionMessage {
                        role: role.clone(),
                        content_preview,
                        model: mdl.clone(),
                        input_tokens: inp_tok,
                        output_tokens: out_tok,
                        timestamp: ts,
                        stop_reason: None,
                    });
                }
                if let Some(inp) = inp_tok {
                    total_input_tokens += inp;
                }
                if let Some(out) = out_tok {
                    total_output_tokens += out;
                }
            }
        }

        let status = if active_set.contains(&session_id) {
            "active"
        } else {
            "completed"
        };

        // Check for subagents
        let session_dir = jsonl_path.with_extension("");
        let subagents_dir = session_dir.join("subagents");
        let mut subagents: Vec<SubAgentInfo> = Vec::new();

        if subagents_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&subagents_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                            if name.ends_with(".meta") {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    if let Ok(meta) =
                                        serde_json::from_str::<serde_json::Value>(&content)
                                    {
                                        subagents.push(SubAgentInfo {
                                            agent_type: meta
                                                .get("agentType")
                                                .and_then(|t| t.as_str())
                                                .unwrap_or("unknown")
                                                .to_string(),
                                            description: meta
                                                .get("description")
                                                .and_then(|d| d.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            message_count: 0,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let display_prompt = first_prompt.map(|p| {
            if p.len() > 200 {
                format!("{}...", &p[..200])
            } else {
                p
            }
        });

        let info = SessionInfo {
            session_id,
            project_path,
            status: status.to_string(),
            executor: executor.unwrap_or_else(|| "unknown".to_string()),
            model: model.unwrap_or_else(|| "-".to_string()),
            git_branch,
            message_count,
            total_input_tokens,
            total_output_tokens,
            first_prompt: display_prompt,
            created_at: first_timestamp,
            last_active_at: last_timestamp,
            file_size,
            version,
            subagent_count: subagents.len() as u32,
        };

        Some(SessionDetail {
            info,
            messages,
            subagents,
        })
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    match detail {
        Some(d) => Ok(ApiResponse::ok(d)),
        None => Err(AppError::NotFound),
    }
}

pub async fn delete_session(
    State(_state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<ApiResponse<()>, AppError> {
    tokio::task::spawn_blocking(move || {
        let projects_dir = claude_home().join("projects");

        if let Ok(project_entries) = std::fs::read_dir(&projects_dir) {
            for project_entry in project_entries.flatten() {
                let jsonl_path = project_entry.path().join(format!("{}.jsonl", session_id));
                if jsonl_path.exists() {
                    // Delete JSONL file
                    let _ = std::fs::remove_file(&jsonl_path);
                    // Delete session directory (subagents, tool-results) if exists
                    let session_dir = jsonl_path.with_extension("");
                    if session_dir.is_dir() {
                        let _ = std::fs::remove_dir_all(&session_dir);
                    }
                    // Delete session-env directory
                    let env_dir = claude_home().join("session-env").join(&session_id);
                    if env_dir.is_dir() {
                        let _ = std::fs::remove_dir_all(&env_dir);
                    }
                    return true;
                }
            }
        }
        false
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(ApiResponse::ok(()))
}
