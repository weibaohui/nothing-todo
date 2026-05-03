use axum::{
    extract::State,
    body::Bytes,
    response::IntoResponse,
    http::header,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

use crate::handlers::{AppError, AppState};
use crate::models::{ApiResponse, BackupData, TagBackup, TodoBackup, utc_timestamp};

/// 导出备份（返回 YAML 格式字符串）
pub async fn export_backup(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let tags = state.db.get_tag_backups().await;
    let todos = state.db.get_todo_backups().await;
    let data = BackupData {
        version: "1.0".to_string(),
        created_at: utc_timestamp(),
        tags,
        todos,
    };
    let yaml = serde_yaml::to_string(&data).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((
        [(header::CONTENT_TYPE, "application/x-yaml; charset=utf-8")],
        yaml,
    ))
}

/// 选择性导出（按 todo ID 列表导出）
#[derive(Deserialize)]
pub struct ExportSelectedRequest {
    pub todo_ids: Vec<i64>,
}

pub async fn export_selected(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<ExportSelectedRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.todo_ids.is_empty() {
        return Err(AppError::BadRequest("No todo IDs provided".to_string()));
    }
    let tags = state.db.get_tag_backups_for_todos(&req.todo_ids).await;
    let todos = state.db.get_todo_backups_by_ids(&req.todo_ids).await;
    if todos.is_empty() {
        return Err(AppError::BadRequest("No todos found for given IDs".to_string()));
    }
    let data = BackupData {
        version: "1.0".to_string(),
        created_at: utc_timestamp(),
        tags,
        todos,
    };
    let yaml = serde_yaml::to_string(&data).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((
        [(header::CONTENT_TYPE, "application/x-yaml; charset=utf-8")],
        yaml,
    ))
}

/// 导入备份（接收 YAML 格式字符串，清空现有数据后导入）
pub async fn import_backup(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<ApiResponse<String>, AppError> {
    let yaml_str = String::from_utf8(body.to_vec())
        .map_err(|_| AppError::BadRequest("Invalid UTF-8 in request body".to_string()))?;
    let data: BackupData = serde_yaml::from_str(&yaml_str)
        .map_err(|e| AppError::BadRequest(format!("Invalid YAML: {}", e)))?;

    if data.todos.is_empty() {
        return Err(AppError::BadRequest("Backup contains no todos".to_string()));
    }

    state.db.import_backup(&data.tags, &data.todos).await
        .map_err(|e| AppError::Internal(format!("Import failed, data unchanged: {}", e)))?;

    Ok(ApiResponse::ok(format!("Imported {} todos and {} tags", data.todos.len(), data.tags.len())))
}

#[derive(Deserialize)]
pub struct MergeRequest {
    pub tags: Vec<TagBackup>,
    pub todos: Vec<TodoBackup>,
}

/// 智能合并导入（不清空现有数据，按 title+prompt 匹配覆盖或新建）
pub async fn merge_backup(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<MergeRequest>,
) -> Result<ApiResponse<String>, AppError> {
    if req.todos.is_empty() {
        return Err(AppError::BadRequest("No todos to merge".to_string()));
    }

    let (created, updated) = state.db.merge_backup(&req.tags, &req.todos).await
        .map_err(|e| AppError::Internal(format!("Merge failed: {}", e)))?;

    Ok(ApiResponse::ok(format!("新建 {} 项，覆盖 {} 项", created, updated)))
}

// ============ 数据库备份 ============

fn backup_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".ntd").join("backups")
}

/// 手动下载数据库文件
pub async fn download_database(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let cfg = crate::config::Config::load();
    let db_path = PathBuf::from(&cfg.db_path);

    if !db_path.exists() {
        return Err(AppError::Internal("Database file not found".to_string()));
    }

    let bytes = std::fs::read(&db_path)
        .map_err(|e| AppError::Internal(format!("Failed to read database: {}", e)))?;

    let filename = format!("ntd-database-{}.db",
        chrono::Utc::now().format("%Y%m%d-%H%M%S"));

    let disposition = format!("attachment; filename=\"{}\"", filename);
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    ))
}

/// 将数据库复制到备份目录
pub async fn trigger_local_backup() -> Result<ApiResponse<String>, AppError> {
    let cfg = crate::config::Config::load();
    let db_path = PathBuf::from(&cfg.db_path);

    if !db_path.exists() {
        return Err(AppError::Internal("Database file not found".to_string()));
    }

    let dir = backup_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Internal(format!("Failed to create backup dir: {}", e)))?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path = dir.join(format!("ntd-backup-{}.db", timestamp));

    std::fs::copy(&db_path, &backup_path)
        .map_err(|e| AppError::Internal(format!("Failed to copy database: {}", e)))?;

    // 清理旧备份，保留最近 30 个
    cleanup_old_backups(30);

    Ok(ApiResponse::ok(format!("备份成功: {}", backup_path.display())))
}

#[derive(Serialize)]
pub struct BackupFile {
    pub name: String,
    pub size: u64,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct BackupStatus {
    pub auto_backup_enabled: bool,
    pub auto_backup_cron: String,
    pub last_backup: Option<String>,
    pub files: Vec<BackupFile>,
}

/// 获取数据库备份状态
pub async fn get_database_backup_status() -> Result<ApiResponse<BackupStatus>, AppError> {
    let cfg = crate::config::Config::load();
    let dir = backup_dir();

    let mut files = Vec::new();
    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "db") {
                    let meta = entry.metadata().ok();
                    let created = meta.as_ref()
                        .and_then(|m| m.created().ok())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Local> = t.into();
                            dt.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_default();
                    files.push(BackupFile {
                        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        size: meta.map(|m| m.len()).unwrap_or(0),
                        created_at: created,
                    });
                }
            }
        }
    }
    files.sort_by(|a, b| b.name.cmp(&a.name));

    let last_backup = files.first().map(|f| f.created_at.clone());

    Ok(ApiResponse::ok(BackupStatus {
        auto_backup_enabled: cfg.auto_backup_enabled,
        auto_backup_cron: cfg.auto_backup_cron,
        last_backup,
        files,
    }))
}

/// 更新自动备份配置
#[derive(Deserialize)]
pub struct UpdateAutoBackupRequest {
    pub enabled: bool,
    pub cron: String,
}

pub async fn update_auto_backup(
    axum::Json(req): axum::Json<UpdateAutoBackupRequest>,
) -> Result<ApiResponse<String>, AppError> {
    // 验证 cron 表达式
    if req.enabled {
        let schedule = cron::Schedule::from_str(&req.cron)
            .map_err(|e| AppError::BadRequest(format!("Invalid cron expression: {}", e)))?;
        // 验证能产生下一次执行时间
        schedule.upcoming(chrono::Utc).next()
            .ok_or_else(|| AppError::BadRequest("Cron expression has no future executions".to_string()))?;
    }

    let mut cfg = crate::config::Config::load();
    cfg.auto_backup_enabled = req.enabled;
    cfg.auto_backup_cron = req.cron;
    cfg.save().map_err(|e| AppError::Internal(e))?;

    Ok(ApiResponse::ok("自动备份配置已更新".to_string()))
}

/// 删除备份文件
#[derive(Deserialize)]
pub struct DeleteBackupRequest {
    pub filename: String,
}

pub async fn delete_backup_file(
    axum::Json(req): axum::Json<DeleteBackupRequest>,
) -> Result<ApiResponse<String>, AppError> {
    // 安全检查：文件名不能包含路径分隔符
    if req.filename.contains('/') || req.filename.contains('\\') || req.filename.contains("..") {
        return Err(AppError::BadRequest("Invalid filename".to_string()));
    }
    let path = backup_dir().join(&req.filename);
    if !path.exists() {
        return Err(AppError::NotFound);
    }
    std::fs::remove_file(&path)
        .map_err(|e| AppError::Internal(format!("Failed to delete: {}", e)))?;
    Ok(ApiResponse::ok("已删除".to_string()))
}

/// 执行数据库文件备份（供定时任务调用）
pub fn perform_database_backup() -> Result<String, String> {
    let cfg = crate::config::Config::load();
    let db_path = PathBuf::from(&cfg.db_path);

    if !db_path.exists() {
        return Err("Database file not found".to_string());
    }

    let dir = backup_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create backup dir: {}", e))?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path = dir.join(format!("ntd-backup-{}.db", timestamp));

    std::fs::copy(&db_path, &backup_path)
        .map_err(|e| format!("Failed to copy database: {}", e))?;

    cleanup_old_backups(30);

    Ok(format!("Auto backup: {}", backup_path.display()))
}

fn cleanup_old_backups(keep: usize) {
    let dir = backup_dir();
    if !dir.exists() {
        return;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "db"))
                .collect()
        })
        .unwrap_or_default();

    if files.len() <= keep {
        return;
    }

    files.sort_by(|a, b| {
        let a_time = std::fs::metadata(a).and_then(|m| m.created()).ok();
        let b_time = std::fs::metadata(b).and_then(|m| m.created()).ok();
        b_time.cmp(&a_time)
    });

    for old_file in files.iter().skip(keep) {
        std::fs::remove_file(old_file).ok();
    }
}

/// 启动自动备份定时任务
pub fn start_auto_backup(cron_expr: &str) -> Result<(), String> {
    let schedule = cron::Schedule::from_str(cron_expr)
        .map_err(|e| format!("Invalid cron: {}", e))?;

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            loop {
                let next = schedule.upcoming(chrono::Utc).next();
                let delay = match next {
                    Some(dt) => {
                        let now = chrono::Utc::now();
                        (dt - now).to_std().unwrap_or(std::time::Duration::from_secs(60))
                    }
                    None => std::time::Duration::from_secs(3600),
                };
                tokio::time::sleep(delay).await;
                match perform_database_backup() {
                    Ok(msg) => tracing::info!("{}", msg),
                    Err(e) => tracing::error!("Auto backup failed: {}", e),
                }
            }
        });
    });

    Ok(())
}
