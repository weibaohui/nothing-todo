//! Usage statistics service
//!
//! Reads usage data from various AI code editor databases and aggregates statistics.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use crate::db::Database;

/// Represents aggregated usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStat {
    pub date: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub extra_total_tokens: i64,
    pub total_cost: f64,
    pub credits: Option<f64>,
    pub message_count: Option<i64>,
    pub models_used: Vec<String>,
    pub project: Option<String>,
    pub last_activity: Option<String>,
    pub stats_type: String,
}

/// Model breakdown for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBreakdown {
    pub model_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub extra_total_tokens: i64,
    pub cost: f64,
}

/// Complete usage report with breakdowns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub daily: Vec<UsageStat>,
    pub weekly: Vec<UsageStat>,
    pub monthly: Vec<UsageStat>,
}

/// Raw usage entry parsed from an editor database
#[derive(Debug, Clone)]
pub struct RawUsageEntry {
    pub timestamp: i64,
    pub date: String,
    pub session_id: String,
    pub project_path: String,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub extra_total_tokens: u64,
    pub cost: f64,
}

/// Statistics collector
#[derive(Default)]
struct TokenAccumulator {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    extra_total_tokens: u64,
    cost: f64,
    models: HashMap<String, ModelAccumulator>,
}

impl TokenAccumulator {
    fn add_entry(&mut self, entry: &RawUsageEntry) {
        self.input_tokens += entry.input_tokens;
        self.output_tokens += entry.output_tokens;
        self.cache_creation_tokens += entry.cache_creation_tokens;
        self.cache_read_tokens += entry.cache_read_tokens;
        self.extra_total_tokens += entry.extra_total_tokens;
        self.cost += entry.cost;

        if let Some(ref model) = entry.model {
            let acc = self.models.entry(model.clone()).or_default();
            acc.input_tokens += entry.input_tokens;
            acc.output_tokens += entry.output_tokens;
            acc.cache_creation_tokens += entry.cache_creation_tokens;
            acc.cache_read_tokens += entry.cache_read_tokens;
            acc.extra_total_tokens += entry.extra_total_tokens;
            acc.cost += entry.cost;
        }
    }

    fn into_usage_stat(self, date: &str, stats_type: &str) -> UsageStat {
        let models_used: Vec<String> = self.models.keys().cloned().collect();
        UsageStat {
            date: date.to_string(),
            input_tokens: self.input_tokens as i64,
            output_tokens: self.output_tokens as i64,
            cache_creation_tokens: self.cache_creation_tokens as i64,
            cache_read_tokens: self.cache_read_tokens as i64,
            extra_total_tokens: self.extra_total_tokens as i64,
            total_cost: self.cost,
            credits: None,
            message_count: None,
            models_used,
            project: None,
            last_activity: None,
            stats_type: stats_type.to_string(),
        }
    }

    fn into_model_breakdowns(self) -> Vec<ModelBreakdown> {
        self.models
            .into_iter()
            .map(|(model_name, acc)| ModelBreakdown {
                model_name,
                input_tokens: acc.input_tokens as i64,
                output_tokens: acc.output_tokens as i64,
                cache_creation_tokens: acc.cache_creation_tokens as i64,
                cache_read_tokens: acc.cache_read_tokens as i64,
                extra_total_tokens: acc.extra_total_tokens as i64,
                cost: acc.cost,
            })
            .collect()
    }
}

#[derive(Default)]
struct ModelAccumulator {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    extra_total_tokens: u64,
    cost: f64,
}

/// Usage statistics service
pub struct UsageStatsService {
    db: Arc<Database>,
}

impl UsageStatsService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get all known editor database paths and JSON cache files
    fn get_editor_db_paths() -> Vec<(String, PathBuf)> {
        let mut paths = Vec::new();

        // Claude Code (claude-code) - ~/.claude/codex.db
        if let Some(home) = dirs::home_dir() {
            let db_path = home.join(".claude").join("codex.db");
            if db_path.exists() {
                paths.push(("claude-code".to_string(), db_path));
            }
            // Also check for stats-cache.json (alternative Claude Code data store)
            let cache_path = home.join(".claude").join("stats-cache.json");
            if cache_path.exists() {
                paths.push(("claude-code-json".to_string(), cache_path));
            }
        }

        // OpenCode (opencode) - typically ~/.opencode/db.sqlite
        if let Some(home) = dirs::home_dir() {
            let db_path = home.join(".opencode").join("db.sqlite");
            if db_path.exists() {
                paths.push(("opencode".to_string(), db_path));
            }

            // Alternative location
            let db_path2 = home.join(".opencode").join("data.db");
            if db_path2.exists() {
                paths.push(("opencode".to_string(), db_path2));
            }
        }

        // Cursor (cursor) - might be in app data
        if let Some(data_dir) = dirs::data_dir() {
            let db_path = data_dir.join("Cursor").join("User").join("globalStorage")
                .join("fdb.kubernetes").join("projects").join(".sqlite");
            if db_path.exists() {
                paths.push(("cursor".to_string(), db_path));
            }
        }

        // VS Code Copilot - usually in extension storage
        if let Some(data_dir) = dirs::data_dir() {
            let copilot_paths = [
                data_dir.join("Code").join("CachedData").join("*"),
                data_dir.join("VSCode").join("CachedData").join("*"),
            ];
            for p in &copilot_paths {
                if p.exists() {
                    paths.push(("copilot".to_string(), p.clone()));
                }
            }
        }

        // Windsurf (windsurf) - ~/.windsurf/db.sqlite
        if let Some(home) = dirs::home_dir() {
            let db_path = home.join(".windsurf").join("db.sqlite");
            if db_path.exists() {
                paths.push(("windsurf".to_string(), db_path));
            }
        }

        // Continue.dev - ~/.continue/db.sqlite
        if let Some(home) = dirs::home_dir() {
            let db_path = home.join(".continue").join("db.sqlite");
            if db_path.exists() {
                paths.push(("continue".to_string(), db_path));
            }
        }

        paths
    }

    /// Load and parse entries from a database or JSON cache file
    async fn load_entries_from_db(&self, db_path: &PathBuf) -> Vec<RawUsageEntry> {
        let mut entries = Vec::new();

        // Check if it's a JSON file (stats-cache.json format)
        if db_path.extension().map(|e| e == "json").unwrap_or(false) {
            return self.load_entries_from_json_cache(db_path).await;
        }

        // Try to read the database using sqlite directly via rusqlite (bundled with libsqlite3-sys)
        let conn = match rusqlite::Connection::open(db_path) {
            Ok(conn) => conn,
            Err(e) => {
                tracing::debug!("Failed to open database {:?}: {}", db_path, e);
                return entries;
            }
        };

        // Try to query for usage data
        // The exact schema varies by editor, so we try multiple common schemas

        // Try Claude protocol format
        let mut stmt = match conn.prepare(
            "SELECT id, session_id, data FROM message WHERE role = 'assistant'"
        ) {
            Ok(stmt) => stmt,
            Err(_) => {
                return entries;
            }
        };

        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let session_id: String = row.get(1)?;
                let data: String = row.get(2)?;
                Ok((id, session_id, data))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        for row in rows {
            if let Some(entry) = self.parse_claude_message(&row.0, &row.1, &row.2) {
                entries.push(entry);
            }
        }

        // Try OpenAI format (used by some editors)
        if entries.is_empty() {
            let mut stmt = match conn.prepare(
                "SELECT created_at, model, usage, cost FROM usage WHERE created_at IS NOT NULL"
            ) {
                Ok(stmt) => stmt,
                Err(_) => {
                    return entries;
                }
            };

            let rows: Vec<(String, String, String, f64)> = stmt
                .query_map([], |row| {
                    let created_at: String = row.get(0)?;
                    let model: String = row.get(1)?;
                    let usage: String = row.get(2)?;
                    let cost: f64 = row.get(3).unwrap_or(0.0);
                    Ok((created_at, model, usage, cost))
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();

            for row in rows {
                if let Some(entry) = self.parse_openai_usage(&row.0, &row.1, &row.2, row.3) {
                    entries.push(entry);
                }
            }
        }

        entries.sort_by_key(|e| e.timestamp);
        entries
    }

    /// Load entries from Claude Code's stats-cache.json
    async fn load_entries_from_json_cache(&self, path: &PathBuf) -> Vec<RawUsageEntry> {
        let mut entries = Vec::new();

        let content = match tokio::fs::read_to_string(path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!("Failed to read JSON cache {:?}: {}", path, e);
                return entries;
            }
        };

        #[derive(Deserialize)]
        struct StatsCache {
            #[serde(rename = "dailyModelTokens")]
            daily_model_tokens: Option<Vec<DailyModelTokens>>,
            #[serde(rename = "modelUsage")]
            model_usage: Option<ModelUsage>,
        }

        #[derive(Deserialize)]
        struct DailyModelTokens {
            date: String,
            #[serde(rename = "tokensByModel")]
            tokens_by_model: std::collections::HashMap<String, i64>,
        }

        #[derive(Deserialize)]
        struct ModelUsage {
            #[serde(flatten)]
            models: std::collections::HashMap<String, ModelStats>,
        }

        #[derive(Deserialize)]
        struct ModelStats {
            #[serde(rename = "inputTokens")]
            input_tokens: Option<i64>,
            #[serde(rename = "outputTokens")]
            output_tokens: Option<i64>,
            #[serde(rename = "cacheReadInputTokens")]
            cache_read_input_tokens: Option<i64>,
            #[serde(rename = "cacheCreationInputTokens")]
            cache_creation_input_tokens: Option<i64>,
            #[serde(rename = "costUSD")]
            cost_usd: Option<f64>,
        }

        let cache: StatsCache = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!("Failed to parse JSON cache: {}", e);
                return entries;
            }
        };

        // Create entries from daily_model_tokens (daily breakdown)
        if let Some(daily) = cache.daily_model_tokens {
            for day_data in daily {
                for (model, tokens) in day_data.tokens_by_model {
                    // Parse date to timestamp
                    let timestamp = chrono::NaiveDate::parse_from_str(&day_data.date, "%Y-%m-%d")
                        .map(|d| d.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc()).unwrap_or_default().timestamp_millis())
                        .unwrap_or(0);

                    entries.push(RawUsageEntry {
                        timestamp,
                        date: day_data.date.clone(),
                        session_id: "cached".to_string(),
                        project_path: "unknown".to_string(),
                        model: Some(model),
                        input_tokens: tokens as u64,
                        output_tokens: 0,
                        cache_creation_tokens: 0,
                        cache_read_tokens: 0,
                        extra_total_tokens: 0,
                        cost: 0.0,
                    });
                }
            }
        }

        // Also create entries from model_usage (aggregate data, using "total" as date)
        if let Some(usage) = cache.model_usage {
            for (model, stats) in usage.models {
                entries.push(RawUsageEntry {
                    timestamp: 0, // Aggregate, no specific date
                    date: "total".to_string(),
                    session_id: "cached".to_string(),
                    project_path: "unknown".to_string(),
                    model: Some(model),
                    input_tokens: stats.input_tokens.unwrap_or(0) as u64,
                    output_tokens: stats.output_tokens.unwrap_or(0) as u64,
                    cache_creation_tokens: stats.cache_creation_input_tokens.unwrap_or(0) as u64,
                    cache_read_tokens: stats.cache_read_input_tokens.unwrap_or(0) as u64,
                    extra_total_tokens: 0,
                    cost: stats.cost_usd.unwrap_or(0.0),
                });
            }
        }

        entries
    }

    /// Parse a Claude protocol message
    fn parse_claude_message(&self, id: &str, session_id: &str, data: &str) -> Option<RawUsageEntry> {
        let json: serde_json::Value = serde_json::from_str(data).ok()?;

        // Extract timestamp
        let timestamp = json.get("created_at")
            .and_then(|v| v.as_i64())
            .or_else(|| json.get("timestamp").and_then(|v| v.as_i64()))?;

        let date = Self::format_timestamp_date(timestamp);
        let model = json.get("model").and_then(|v| v.as_str()).map(String::from);

        // Extract usage
        let usage_obj = json.get("usage")?;
        let input_tokens = usage_obj.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let output_tokens = usage_obj.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cache_creation = usage_obj.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cache_read = usage_obj.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

        // Extra tokens (reasoning, etc)
        let reasoning = usage_obj.get("reasoning_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let extra = reasoning;

        let cost = json.get("cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);

        Some(RawUsageEntry {
            timestamp,
            date,
            session_id: session_id.to_string(),
            project_path: "unknown".to_string(),
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens: cache_creation,
            cache_read_tokens: cache_read,
            extra_total_tokens: extra,
            cost,
        })
    }

    /// Parse OpenAI-style usage
    fn parse_openai_usage(&self, created_at: &str, model: &str, usage: &str, cost: f64) -> Option<RawUsageEntry> {
        let usage_json: serde_json::Value = serde_json::from_str(usage).ok()?;

        let timestamp = Self::parse_timestamp(created_at)?;
        let date = Self::format_timestamp_date(timestamp);

        let input_tokens = usage_json.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let output_tokens = usage_json.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cache_creation = usage_json.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cache_read = usage_json.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

        Some(RawUsageEntry {
            timestamp,
            date,
            session_id: "unknown".to_string(),
            project_path: "unknown".to_string(),
            model: Some(model.to_string()),
            input_tokens,
            output_tokens,
            cache_creation_tokens: cache_creation,
            cache_read_tokens: cache_read,
            extra_total_tokens: 0,
            cost,
        })
    }

    /// Parse timestamp to i64 milliseconds
    fn parse_timestamp(s: &str) -> Option<i64> {
        // Try ISO 8601 format
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp_millis());
        }

        // Try Unix timestamp
        if let Ok(ts) = s.parse::<i64>() {
            return Some(ts * 1000);
        }

        None
    }

    /// Format timestamp to YYYY-MM-DD
    fn format_timestamp_date(timestamp_ms: i64) -> String {
        let secs = timestamp_ms / 1000;
        match chrono::DateTime::from_timestamp(secs, 0) {
            Some(dt) => dt.format("%Y-%m-%d").to_string(),
            None => String::new(),
        }
    }

    /// Collect all entries from all editor databases
    pub async fn collect_all_entries(&self) -> Vec<RawUsageEntry> {
        let paths = Self::get_editor_db_paths();
        let mut all_entries = Vec::new();

        for (editor, path) in paths {
            tracing::debug!("Loading entries from {} at {:?}", editor, path);
            let entries = self.load_entries_from_db(&path).await;
            tracing::info!("Loaded {} entries from {}", entries.len(), editor);
            all_entries.extend(entries);
        }

        // Sort by timestamp
        all_entries.sort_by_key(|e| e.timestamp);
        all_entries
    }

    /// Aggregate entries by day
    pub fn aggregate_by_day(entries: &[RawUsageEntry]) -> Vec<UsageStat> {
        let mut daily_map: HashMap<String, TokenAccumulator> = HashMap::new();

        for entry in entries {
            let acc = daily_map.entry(entry.date.clone()).or_default();
            acc.add_entry(entry);
        }

        daily_map
            .into_iter()
            .map(|(date, acc)| acc.into_usage_stat(&date, "daily"))
            .collect()
    }

    /// Aggregate daily stats into weekly
    fn aggregate_by_week(daily: &[UsageStat]) -> Vec<UsageStat> {
        let mut weekly_map: HashMap<String, TokenAccumulator> = HashMap::new();

        for stat in daily {
            let week_start = Self::get_week_start(&stat.date);
            let acc = weekly_map.entry(week_start).or_default();
            acc.input_tokens += stat.input_tokens as u64;
            acc.output_tokens += stat.output_tokens as u64;
            acc.cache_creation_tokens += stat.cache_creation_tokens as u64;
            acc.cache_read_tokens += stat.cache_read_tokens as u64;
            acc.extra_total_tokens += stat.extra_total_tokens as u64;
            acc.cost += stat.total_cost;
        }

        weekly_map
            .into_iter()
            .map(|(date, acc)| acc.into_usage_stat(&date, "weekly"))
            .collect()
    }

    /// Aggregate daily stats into monthly
    fn aggregate_by_month(daily: &[UsageStat]) -> Vec<UsageStat> {
        let mut monthly_map: HashMap<String, TokenAccumulator> = HashMap::new();

        for stat in daily {
            let month = stat.date[..7].to_string(); // YYYY-MM
            let acc = monthly_map.entry(month).or_default();
            acc.input_tokens += stat.input_tokens as u64;
            acc.output_tokens += stat.output_tokens as u64;
            acc.cache_creation_tokens += stat.cache_creation_tokens as u64;
            acc.cache_read_tokens += stat.cache_read_tokens as u64;
            acc.extra_total_tokens += stat.extra_total_tokens as u64;
            acc.cost += stat.total_cost;
        }

        monthly_map
            .into_iter()
            .map(|(date, acc)| acc.into_usage_stat(&date, "monthly"))
            .collect()
    }

    /// Get the start of the week (Monday) for a given date
    fn get_week_start(date: &str) -> String {
        let dt = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok();
        if let Some(dt) = dt {
            let days_since_monday = dt.weekday().num_days_from_monday();
            let monday = dt - chrono::Duration::days(days_since_monday as i64);
            monday.format("%Y-%m-%d").to_string()
        } else {
            date.to_string()
        }
    }

    /// Save aggregated stats to database
    pub async fn save_daily_stats(&self, stats: &[UsageStat]) -> Result<(), String> {
        for stat in stats {
            // Check if we already have this date's stats
            let existing = self.db.get_latest_usage_stat(&stat.date, "daily").await
                .map_err(|e| e.to_string())?;

            if existing.is_some() {
                // Delete and re-insert (update)
                self.db.delete_usage_stats_by_date(&stat.date, "daily").await
                    .map_err(|e| e.to_string())?;
            }

            // Insert new stats
            let stat_id = self.db.create_usage_daily_stat(
                &stat.date,
                stat.project.as_deref(),
                None,
                stat.input_tokens,
                stat.output_tokens,
                stat.cache_creation_tokens,
                stat.cache_read_tokens,
                stat.extra_total_tokens,
                stat.total_cost,
                stat.credits,
                stat.message_count,
                &stat.models_used,
                stat.project.as_deref(),
                None,
                stat.last_activity.as_deref(),
                "daily",
            ).await.map_err(|e| e.to_string())?;

            // Insert model breakdowns
            // We need to recalculate them from the stat
            let mut acc = TokenAccumulator::default();
            for model_name in &stat.models_used {
                acc.models.insert(model_name.clone(), ModelAccumulator {
                    input_tokens: 0, // These would need to be tracked separately
                    output_tokens: 0,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 0,
                    extra_total_tokens: 0,
                    cost: 0.0,
                });
            }
            // Note: Model breakdowns would need to be stored separately during aggregation
            // For now, we just store the daily totals
            let _ = stat_id; // Silence unused warning
        }

        Ok(())
    }

    /// Generate and store today's real-time stats
    pub async fn update_today_stats(&self) -> Result<UsageReport, String> {
        let entries = self.collect_all_entries().await;

        if entries.is_empty() {
            return Ok(UsageReport {
                daily: vec![],
                weekly: vec![],
                monthly: vec![],
            });
        }

        // Get today's date
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        // Filter entries for today
        let today_entries: Vec<_> = entries.iter()
            .filter(|e| e.date == today)
            .cloned()
            .collect();

        // Aggregate today's entries
        let daily = if today_entries.is_empty() {
            vec![]
        } else {
            let mut acc = TokenAccumulator::default();
            for entry in &today_entries {
                acc.add_entry(entry);
            }
            vec![acc.into_usage_stat(&today, "daily")]
        };

        // For weekly and monthly, we need historical data
        // For now, just use today's entries aggregated up
        let weekly = Self::aggregate_by_week(&daily);
        let monthly = Self::aggregate_by_month(&daily);

        // Save today's stats
        if !daily.is_empty() {
            self.save_daily_stats(&daily).await?;
        }

        Ok(UsageReport {
            daily,
            weekly,
            monthly,
        })
    }

    /// Refresh all stats from all sources (including historical data)
    pub async fn refresh_all_stats(&self) -> Result<UsageReport, String> {
        let entries = self.collect_all_entries().await;

        if entries.is_empty() {
            return Ok(UsageReport {
                daily: vec![],
                weekly: vec![],
                monthly: vec![],
            });
        }

        // Filter out entries with "total" date (those are aggregates)
        let dated_entries: Vec<_> = entries.iter()
            .filter(|e| e.date != "total")
            .cloned()
            .collect();

        if dated_entries.is_empty() {
            return Ok(UsageReport {
                daily: vec![],
                weekly: vec![],
                monthly: vec![],
            });
        }

        // Aggregate all dated entries by day
        let daily = Self::aggregate_by_day(&dated_entries);

        // Save all daily stats
        if !daily.is_empty() {
            self.save_daily_stats(&daily).await?;
        }

        // Aggregate into weekly and monthly
        let weekly = Self::aggregate_by_week(&daily);
        let monthly = Self::aggregate_by_month(&daily);

        Ok(UsageReport {
            daily,
            weekly,
            monthly,
        })
    }

    /// Get historical stats from database
    pub async fn get_stats(
        &self,
        stats_type: &str,
        since: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<UsageStat>, String> {
        // For weekly/monthly, compute from daily data stored in DB
        if stats_type == "weekly" || stats_type == "monthly" {
            // Get all daily stats in the date range
            let daily_models = self.db.get_usage_stats("daily", since, until)
                .await
                .map_err(|e| e.to_string())?;

            if daily_models.is_empty() {
                return Ok(vec![]);
            }

            let daily: Vec<UsageStat> = daily_models.into_iter().map(|m| UsageStat {
                date: m.date,
                input_tokens: m.input_tokens,
                output_tokens: m.output_tokens,
                cache_creation_tokens: m.cache_creation_tokens,
                cache_read_tokens: m.cache_read_tokens,
                extra_total_tokens: m.extra_total_tokens,
                total_cost: m.total_cost,
                credits: m.credits,
                message_count: m.message_count,
                models_used: serde_json::from_str(&m.models_used).unwrap_or_default(),
                project: m.project,
                last_activity: m.last_activity,
                stats_type: m.stats_type,
            }).collect();

            return if stats_type == "weekly" {
                Ok(Self::aggregate_by_week(&daily))
            } else {
                Ok(Self::aggregate_by_month(&daily))
            };
        }

        // For daily, query directly from database
        let models = self.db.get_usage_stats(stats_type, since, until)
            .await
            .map_err(|e| e.to_string())?;

        Ok(models.into_iter().map(|m| UsageStat {
            date: m.date,
            input_tokens: m.input_tokens,
            output_tokens: m.output_tokens,
            cache_creation_tokens: m.cache_creation_tokens,
            cache_read_tokens: m.cache_read_tokens,
            extra_total_tokens: m.extra_total_tokens,
            total_cost: m.total_cost,
            credits: m.credits,
            message_count: m.message_count,
            models_used: serde_json::from_str(&m.models_used).unwrap_or_default(),
            project: m.project,
            last_activity: m.last_activity,
            stats_type: m.stats_type,
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_week_start() {
        // Monday 2024-01-08 should return 2024-01-08
        assert_eq!(UsageStatsService::get_week_start("2024-01-08"), "2024-01-08");
        // Tuesday 2024-01-09 should return 2024-01-08
        assert_eq!(UsageStatsService::get_week_start("2024-01-09"), "2024-01-08");
        // Sunday 2024-01-14 should return 2024-01-08
        assert_eq!(UsageStatsService::get_week_start("2024-01-14"), "2024-01-08");
        // Monday 2024-01-15 should return 2024-01-15
        assert_eq!(UsageStatsService::get_week_start("2024-01-15"), "2024-01-15");
    }
}
