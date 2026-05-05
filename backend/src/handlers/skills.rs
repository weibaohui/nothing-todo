//! Skills management handler.
//!
//! Discovers skills from executor directories, provides comparison, sync,
//! and execution tracking APIs.

use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::Write;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

use crate::models::ExecutorType;
use crate::handlers::{AppError, AppState, ApiJson};
use crate::models::ApiResponse;

// ── Data types ──────────────────────────────────────────────────────────

/// Executor type → skills directory mapping
fn executor_skills_dir(et: ExecutorType) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match et {
        ExecutorType::Claudecode => Some(home.join(".claude").join("skills")),
        ExecutorType::Hermes => Some(home.join(".hermes").join("skills")),
        ExecutorType::Codex => Some(home.join(".codex").join("skills")),
        ExecutorType::Codebuddy => Some(home.join(".codebuddy").join("skills")),
        ExecutorType::Opencode => Some(home.join(".opencode").join("skills")),
        ExecutorType::Atomcode => Some(home.join(".atomcode").join("skills")),
        ExecutorType::Kimi => Some(home.join(".kimi").join("skills")),
        ExecutorType::Joinai => Some(home.join(".joinai").join("skills")),
    }
}

fn executor_label(et: ExecutorType) -> &'static str {
    match et {
        ExecutorType::Claudecode => "Claude Code",
        ExecutorType::Hermes => "Hermes",
        ExecutorType::Codex => "Codex",
        ExecutorType::Codebuddy => "CodeBuddy",
        ExecutorType::Opencode => "Opencode",
        ExecutorType::Atomcode => "AtomCode",
        ExecutorType::Kimi => "Kimi",
        ExecutorType::Joinai => "JoinAI",
    }
}

const ALL_EXECUTORS: [ExecutorType; 8] = [
    ExecutorType::Claudecode,
    ExecutorType::Hermes,
    ExecutorType::Codex,
    ExecutorType::Codebuddy,
    ExecutorType::Opencode,
    ExecutorType::Atomcode,
    ExecutorType::Kimi,
    ExecutorType::Joinai,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
    pub file_count: u32,
    pub total_size: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorSkills {
    pub executor: String,
    pub executor_label: String,
    pub skills_dir: String,
    pub skills_dir_exists: bool,
    pub skills: Vec<SkillMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillComparison {
    pub skill_name: String,
    pub description: String,
    pub executors: HashMap<String, SkillPresence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPresence {
    pub present: bool,
    pub version: Option<String>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInvocation {
    pub id: i64,
    pub skill_name: String,
    pub executor: String,
    pub todo_id: i64,
    pub todo_title: Option<String>,
    pub invoked_at: String,
    pub status: String,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub source_executor: String,
    pub skill_name: String,
    pub target_executors: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct InvocationQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub skill_name: Option<String>,
    pub executor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecordInvocationRequest {
    pub skill_name: String,
    pub executor: String,
    pub todo_id: i64,
    pub status: String,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SkillContentQuery {
    pub executor: String,
    pub skill_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SkillExportQuery {
    pub executor: String,
    pub skill_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub executor: String,
    pub skill_name: Option<String>,
    pub flatten: Option<bool>,
}

// ── Skill discovery ─────────────────────────────────────────────────────

fn parse_skill_yaml_header(content: &str) -> SkillMeta {
    let mut name = String::new();
    let mut description = String::new();
    let mut version = None;
    let mut author = None;
    let mut license = None;
    let mut keywords = Vec::new();

    // Parse YAML front matter between --- markers
    if let Some(yaml_content) = extract_yaml_front_matter(content) {
        for line in yaml_content.lines() {
            if let Some(val) = line.strip_prefix("name:") {
                name = val.trim().trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("description:") {
                // description can be multi-line or quoted
                let val = val.trim();
                if val.starts_with('|') || val.starts_with('>') {
                    // skip multi-line for now, use first line
                } else {
                    description = val.trim_matches('"').to_string();
                }
            } else if let Some(val) = line.strip_prefix("version:") {
                version = Some(val.trim().trim_matches('"').to_string());
            } else if let Some(val) = line.strip_prefix("author:") {
                author = Some(val.trim().trim_matches('"').to_string());
            } else if let Some(val) = line.strip_prefix("license:") {
                license = Some(val.trim().trim_matches('"').to_string());
            } else if let Some(val) = line.strip_prefix("  - ") {
                if !keywords.is_empty() || line.contains("keywords:") {
                    keywords.push(val.trim_matches('"').to_string());
                }
            }
        }
    }

    // Fallback: if name is empty, try first heading
    if name.is_empty() {
        for line in content.lines() {
            if let Some(heading) = line.strip_prefix("# ") {
                name = heading.trim().to_string();
                break;
            }
        }
    }

    // Fallback: if description is empty, use first non-empty, non-front-matter line
    if description.is_empty() {
        let mut past_front = false;
        let mut dash_count = 0;
        for line in content.lines() {
            if line.trim() == "---" {
                dash_count += 1;
                if dash_count >= 2 {
                    past_front = true;
                }
                continue;
            }
            if past_front && !line.trim().is_empty() && !line.starts_with('#') {
                description = line.trim().chars().take(200).collect();
                break;
            }
        }
    }

    SkillMeta {
        name,
        description,
        version,
        author,
        license,
        keywords,
        file_count: 0,
        total_size: 0,
        modified_at: None,
    }
}

fn extract_yaml_front_matter(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.first()?.trim() != "---" {
        return None;
    }
    let mut end = 1;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end = i;
            break;
        }
    }
    Some(lines[1..end].join("\n"))
}

fn count_files_and_size(dir: &std::path::Path) -> (u32, u64) {
    let mut count = 0u32;
    let mut size = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    count += 1;
                    size += metadata.len();
                } else if metadata.is_dir() {
                    let (c, s) = count_files_and_size(&entry.path());
                    count += c;
                    size += s;
                }
            }
        }
    }
    (count, size)
}

/// Recursively find skill directories containing SKILL.md.
/// Supports both flat (skill/SKILL.md) and nested (category/skill/SKILL.md) layouts.
fn collect_skills_recursive(base_dir: &std::path::Path, current_dir: &std::path::Path, skills: &mut Vec<SkillMeta>) {
    if let Ok(entries) = std::fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                let content = std::fs::read_to_string(&skill_md).unwrap_or_default();
                let mut meta = parse_skill_yaml_header(&content);

                if meta.name.is_empty() {
                    meta.name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                }

                // Use relative path from base as a category prefix for nested dirs
                if let Ok(rel) = path.strip_prefix(base_dir) {
                    let rel_str = rel.to_string_lossy().to_string();
                    // Only add prefix if nested (e.g. "devops/lark-cli" -> keep as name)
                    if rel_str.contains('/') {
                        if meta.name == path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default() {
                            meta.name = rel_str;
                        }
                    }
                }

                let (file_count, total_size) = count_files_and_size(&path);
                meta.file_count = file_count;
                meta.total_size = total_size;

                if let Ok(metadata) = std::fs::metadata(&skill_md) {
                    meta.modified_at = metadata.modified().ok().map(|t| {
                        let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                        chrono::DateTime::from_timestamp(secs as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                            .unwrap_or_default()
                    });
                }

                skills.push(meta);
            } else {
                // No SKILL.md here — recurse deeper (may be a category folder)
                collect_skills_recursive(base_dir, &path, skills);
            }
        }
    }
}

fn discover_skills_for_executor(et: ExecutorType) -> ExecutorSkills {
    let skills_dir = match executor_skills_dir(et) {
        Some(p) => p,
        None => {
            return ExecutorSkills {
                executor: et.as_str().to_string(),
                executor_label: executor_label(et).to_string(),
                skills_dir: String::new(),
                skills_dir_exists: false,
                skills: vec![],
            };
        }
    };

    let dir_str = skills_dir.to_string_lossy().to_string();
    let exists = skills_dir.exists();

    let mut skills = Vec::new();
    if exists {
        collect_skills_recursive(&skills_dir, &skills_dir, &mut skills);
    }

    // Sort skills by name
    skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    ExecutorSkills {
        executor: et.as_str().to_string(),
        executor_label: executor_label(et).to_string(),
        skills_dir: dir_str,
        skills_dir_exists: exists,
        skills,
    }
}

// ── API handlers ────────────────────────────────────────────────────────

/// GET /xyz/skills - List skills grouped by executor
pub async fn list_skills(
    State(_state): State<AppState>,
) -> Result<ApiResponse<Vec<ExecutorSkills>>, AppError> {
    let result: Vec<ExecutorSkills> = ALL_EXECUTORS
        .iter()
        .map(|et| discover_skills_for_executor(*et))
        .collect();
    Ok(ApiResponse::ok(result))
}

/// GET /xyz/skills/content - Get skill content (SKILL.md and metadata)
pub async fn get_skill_content(
    Query(query): Query<SkillContentQuery>,
) -> Result<ApiResponse<SkillContentResponse>, AppError> {
    let et = crate::adapters::parse_executor_type(&query.executor)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown executor: {}", query.executor)))?;

    let skills_dir = executor_skills_dir(et)
        .ok_or_else(|| AppError::BadRequest("No skills directory for this executor".to_string()))?;

    let skill_dir = skills_dir.join(&query.skill_name);
    if !skill_dir.exists() {
        return Err(AppError::NotFound);
    }

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = if skill_md_path.exists() {
        std::fs::read_to_string(&skill_md_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Collect all files in the skill directory
    let mut files = Vec::new();
    collect_skill_files(&skill_dir, &skill_dir, &mut files);

    Ok(ApiResponse::ok(SkillContentResponse {
        skill_name: query.skill_name,
        executor: query.executor,
        content,
        files,
    }))
}

/// GET /xyz/skills/export - Export skill as .tar.gz
pub async fn export_skill(
    Query(query): Query<SkillExportQuery>,
) -> Result<Vec<u8>, AppError> {
    let et = crate::adapters::parse_executor_type(&query.executor)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown executor: {}", query.executor)))?;

    let skills_dir = executor_skills_dir(et)
        .ok_or_else(|| AppError::BadRequest("No skills directory for this executor".to_string()))?;

    let skill_dir = skills_dir.join(&query.skill_name);
    if !skill_dir.exists() {
        return Err(AppError::NotFound);
    }

    // Create tar.gz in memory
    let mut tar_data = Vec::new();
    {
        let encoder = GzEncoder::new(&mut tar_data, Compression::default());
        let mut tar_builder = tar::Builder::new(encoder);
        add_dir_to_tar(&mut tar_builder, &skill_dir, &query.skill_name)
            .map_err(|e| AppError::Internal(format!("Failed to create archive: {}", e)))?;
        tar_builder.finish()
            .map_err(|e| AppError::Internal(format!("Failed to finish archive: {}", e)))?;
    }

    Ok(tar_data)
}

fn add_dir_to_tar<W: Write>(
    builder: &mut tar::Builder<W>,
    dir: &std::path::Path,
    prefix: &str,
) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = format!("{}/{}", prefix, path.file_name().unwrap().to_string_lossy());

        if path.is_dir() {
            add_dir_to_tar(builder, &path, &name)?;
        } else {
            let mut file = std::fs::File::open(&path)?;
            builder.append_file(&name, &mut file)?;
        }
    }

    Ok(())
}

/// POST /xyz/skills/import - Import skill from .tar.gz
pub async fn import_skill(
    State(_state): State<AppState>,
    params: Query<ImportRequest>,
    body: axum::body::Bytes,
) -> Result<ApiResponse<ImportResult>, AppError> {
    let et = crate::adapters::parse_executor_type(&params.executor)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown executor: {}", params.executor)))?;

    let skills_dir = executor_skills_dir(et)
        .ok_or_else(|| AppError::BadRequest("No skills directory for this executor".to_string()))?;

    std::fs::create_dir_all(&skills_dir)
        .map_err(|e| AppError::Internal(format!("Failed to create skills dir: {}", e)))?;

    // Decode tar.gz
    let cursor = std::io::Cursor::new(body.to_vec());
    let decoder = GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(decoder);

    let flatten = params.flatten.unwrap_or(true);
    let skill_name = params.skill_name.clone().unwrap_or_else(|| "imported-skill".to_string());
    let target_dir = skills_dir.join(&skill_name);

    std::fs::create_dir_all(&target_dir)
        .map_err(|e| AppError::Internal(format!("Failed to create target dir: {}", e)))?;

    let mut imported_files = 0i32;
    for entry in archive.entries().map_err(|e| AppError::Internal(format!("Failed to read archive: {}", e)))? {
        let mut entry = entry.map_err(|e| AppError::Internal(format!("Failed to read entry: {}", e)))?;
        let path = entry.path()
            .map_err(|e| AppError::Internal(format!("Invalid path: {}", e)))?
            .into_owned();

        let file_name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip directories and hidden files
        if file_name.starts_with('.') || file_name.is_empty() {
            continue;
        }

        let dest_path = if flatten {
            // Flatten: extract directly to target dir, ignoring subdirectories
            target_dir.join(&file_name)
        } else {
            // Preserve structure
            target_dir.join(&path)
        };

        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Internal(format!("Failed to create dir: {}", e)))?;
        }

        entry.unpack(&dest_path)
            .map_err(|e| AppError::Internal(format!("Failed to extract {}: {}", file_name, e)))?;
        imported_files += 1;
    }

    Ok(ApiResponse::ok(ImportResult {
        skill_name,
        imported_files,
        message: format!("Successfully imported {} files", imported_files),
    }))
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub skill_name: String,
    pub imported_files: i32,
    pub message: String,
}

fn collect_skill_files(base: &std::path::Path, current: &std::path::Path, files: &mut Vec<SkillFileInfo>) {
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let rel_path = path.strip_prefix(base)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    files.push(SkillFileInfo {
                        path: rel_path,
                        size: metadata.len(),
                        modified_at: metadata.modified().ok().map(|t| {
                            let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                            chrono::DateTime::from_timestamp(secs as i64, 0)
                                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                                .unwrap_or_default()
                        }).unwrap_or_default(),
                    });
                } else if metadata.is_dir() {
                    collect_skill_files(base, &path, files);
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SkillContentResponse {
    pub skill_name: String,
    pub executor: String,
    pub content: String,
    pub files: Vec<SkillFileInfo>,
}

#[derive(Debug, Serialize)]
pub struct SkillFileInfo {
    pub path: String,
    pub size: u64,
    pub modified_at: String,
}

/// GET /xyz/skills/compare - Cross-executor skill comparison matrix
pub async fn compare_skills(
    State(_state): State<AppState>,
) -> Result<ApiResponse<Vec<SkillComparison>>, AppError> {
    // Collect all skills per executor
    let mut all_skills: HashMap<String, HashMap<String, SkillMeta>> = HashMap::new();
    for et in &ALL_EXECUTORS {
        let es = discover_skills_for_executor(*et);
        let mut map = HashMap::new();
        for skill in es.skills {
            map.insert(skill.name.clone(), skill);
        }
        all_skills.insert(et.as_str().to_string(), map);
    }

    // Build union of all skill names
    let mut skill_names: Vec<String> = all_skills.values()
        .flat_map(|m| m.keys().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    skill_names.sort();

    // Build comparison
    let comparisons: Vec<SkillComparison> = skill_names.into_iter().map(|name| {
        let mut executors_map = HashMap::new();
        for et in &ALL_EXECUTORS {
            let key = et.as_str().to_string();
            if let Some(skill) = all_skills.get(&key).and_then(|m| m.get(&name)) {
                executors_map.insert(key, SkillPresence {
                    present: true,
                    version: skill.version.clone(),
                    modified_at: skill.modified_at.clone(),
                });
            } else {
                executors_map.insert(key, SkillPresence {
                    present: false,
                    version: None,
                    modified_at: None,
                });
            }
        }

        // Get description from any executor that has it
        let description = all_skills.values()
            .filter_map(|m| m.get(&name))
            .find_map(|s| {
                if s.description.is_empty() { None } else { Some(s.description.clone()) }
            })
            .unwrap_or_default();

        SkillComparison {
            skill_name: name,
            description,
            executors: executors_map,
        }
    }).collect();

    Ok(ApiResponse::ok(comparisons))
}

/// POST /xyz/skills/sync - Sync skill from one executor to others
pub async fn sync_skill(
    State(_state): State<AppState>,
    ApiJson(req): ApiJson<SyncRequest>,
) -> Result<ApiResponse<String>, AppError> {
    let source_et = crate::adapters::parse_executor_type(&req.source_executor)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown executor: {}", req.source_executor)))?;

    let source_dir = executor_skills_dir(source_et)
        .ok_or_else(|| AppError::BadRequest("No skills directory for source executor".to_string()))?;

    let skill_dir = source_dir.join(&req.skill_name);
    if !skill_dir.exists() {
        return Err(AppError::NotFound);
    }

    let mut synced = Vec::new();
    let mut errors = Vec::new();

    for target in &req.target_executors {
        let target_et = match crate::adapters::parse_executor_type(target) {
            Some(et) => et,
            None => {
                errors.push(format!("Unknown target executor: {}", target));
                continue;
            }
        };

        let target_dir = match executor_skills_dir(target_et) {
            Some(d) => d,
            None => {
                errors.push(format!("No skills directory for {}", target));
                continue;
            }
        };

        // Create target skills directory if needed
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| AppError::Internal(format!("Failed to create target dir: {}", e)))?;

        let dest = target_dir.join(&req.skill_name);

        // Remove existing if present
        if dest.exists() {
            std::fs::remove_dir_all(&dest)
                .map_err(|e| AppError::Internal(format!("Failed to remove existing: {}", e)))?;
        }

        // Copy recursively
        match copy_dir_recursive(&skill_dir, &dest) {
            Ok(_) => synced.push(target.clone()),
            Err(e) => errors.push(format!("Failed to sync to {}: {}", target, e)),
        }
    }

    if synced.is_empty() && !errors.is_empty() {
        return Err(AppError::Internal(errors.join("; ")));
    }

    let mut msg = format!("Synced '{}' to: {}", req.skill_name, synced.join(", "));
    if !errors.is_empty() {
        msg.push_str(&format!(" | Errors: {}", errors.join("; ")));
    }

    Ok(ApiResponse::ok(msg))
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// GET /xyz/skills/invocations - List skill invocation records
pub async fn list_invocations(
    State(state): State<AppState>,
    Query(query): Query<InvocationQuery>,
) -> Result<ApiResponse<Vec<SkillInvocation>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    let invocations = state.db.get_skill_invocations(
        offset,
        limit,
        query.skill_name.as_deref(),
        query.executor.as_deref(),
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(ApiResponse::ok(invocations))
}

/// POST /xyz/skills/invocations - Record a skill invocation
pub async fn record_invocation(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<RecordInvocationRequest>,
) -> Result<ApiResponse<i64>, AppError> {
    let id = state.db.record_skill_invocation(
        &req.skill_name,
        &req.executor,
        req.todo_id,
        &req.status,
        req.duration_ms,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(ApiResponse::ok(id))
}
