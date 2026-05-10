use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tokio::sync::{RwLock, broadcast};
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::adapters::ExecutorRegistry;
use crate::config::Config as AppConfig;
use crate::db::Database;
use crate::feishu::sdk::config::{Config as FeishuSdkConfig, CONTENT_TYPE_JSON};
use crate::feishu::sdk::token_manager::TokenManager;
use crate::handlers::ExecEvent;
use crate::handlers::execution::start_todo_execution;
use crate::task_manager::TaskManager;

const IM_V1_LIST_MESSAGES: &str = "/open-apis/im/v1/messages";

pub struct FeishuHistoryFetcher {
    db: Arc<Database>,
    executor_registry: Arc<ExecutorRegistry>,
    tx: broadcast::Sender<ExecEvent>,
    task_manager: Arc<TaskManager>,
    config: Arc<RwLock<AppConfig>>,
    token_manager: Arc<TokenManager>,
    bot_credentials: Arc<DashMap<i64, (String, String, String)>>,
}

#[derive(Debug, Deserialize)]
struct ListMessagesResponse {
    code: i32,
    msg: String,
    data: Option<ListMessagesData>,
}

#[derive(Debug, Deserialize)]
struct ListMessagesData {
    has_more: bool,
    page_token: Option<String>,
    items: Option<Vec<MessageItem>>,
}

#[derive(Debug, Deserialize)]
struct MessageItem {
    message_id: String,
    msg_type: String,
    chat_id: String,
    #[allow(dead_code)]
    chat_type: Option<String>,
    sender: Option<Sender>,
    body: Option<MessageBody>,
    create_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Sender {
    id: Option<String>,
    #[allow(dead_code)]
    id_type: Option<String>,
    #[allow(dead_code)]
    sender_type: Option<String>,
    #[allow(dead_code)]
    tenant_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageBody {
    content: Option<String>,
}

#[derive(Clone, Debug)]
struct ChatToFetch {
    bot_id: i64,
    chat_id: String,
}

impl FeishuHistoryFetcher {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Arc<Database>,
        executor_registry: Arc<ExecutorRegistry>,
        tx: broadcast::Sender<ExecEvent>,
        task_manager: Arc<TaskManager>,
        config: Arc<RwLock<AppConfig>>,
        token_manager: Arc<TokenManager>,
        bot_credentials: Arc<DashMap<i64, (String, String, String)>>,
    ) -> Self {
        Self {
            db,
            executor_registry,
            tx,
            task_manager,
            config,
            token_manager,
            bot_credentials,
        }
    }

    pub fn start(&self, bots: Vec<(i64, String, String)>) {
        if bots.is_empty() {
            info!("[feishu-history-fetcher] no bots configured, skipping");
            return;
        }

        let db = self.db.clone();
        let executor_registry = self.executor_registry.clone();
        let tx = self.tx.clone();
        let task_manager = self.task_manager.clone();
        let config = self.config.clone();
        let token_manager = self.token_manager.clone();
        let bot_credentials = self.bot_credentials.clone();

        tokio::spawn(async move {
            info!("[feishu-history-fetcher] started");
            let mut ticker = interval(Duration::from_secs(60));

            loop {
                ticker.tick().await;

                // Collect all chats to fetch from both sources
                let mut chats_to_fetch: Vec<ChatToFetch> = Vec::new();

                // 1. Get chats from feishu_history_chats table
                for (bot_id, _, _) in &bots {
                    if let Ok(history_chats) = db.get_enabled_feishu_history_chats(*bot_id).await {
                        for chat in history_chats {
                            chats_to_fetch.push(ChatToFetch {
                                bot_id: *bot_id,
                                chat_id: chat.chat_id,
                            });
                        }
                    }
                }

                // 2. Get chats from feishu_push_targets table (group chat only)
                if let Ok(push_targets) = db.get_group_chat_push_targets().await {
                    for (bot_id, chat_id) in push_targets {
                        // Avoid duplicates
                        if !chats_to_fetch.iter().any(|c| c.bot_id == bot_id && c.chat_id == chat_id) {
                            chats_to_fetch.push(ChatToFetch { bot_id, chat_id });
                        }
                    }
                }

                if chats_to_fetch.is_empty() {
                    debug!("[feishu-history-fetcher] no chats to fetch");
                    continue;
                }

                for (bot_id, app_id, app_secret) in &bots {
                    // Filter chats for this bot
                    let bot_chats: Vec<_> = chats_to_fetch.iter()
                        .filter(|c| c.bot_id == *bot_id)
                        .cloned()
                        .collect();

                    if bot_chats.is_empty() {
                        continue;
                    }

                    if let Err(e) = Self::fetch_for_bot(
                        &db,
                        &executor_registry,
                        &tx,
                        &task_manager,
                        &config,
                        &token_manager,
                        &bot_credentials,
                        *bot_id,
                        app_id,
                        app_secret,
                        &bot_chats,
                    ).await {
                        warn!("[feishu-history-fetcher] error fetching for bot {}: {}", bot_id, e);
                    }
                }
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    async fn fetch_for_bot(
        db: &Arc<Database>,
        executor_registry: &Arc<ExecutorRegistry>,
        tx: &broadcast::Sender<ExecEvent>,
        task_manager: &Arc<TaskManager>,
        config: &Arc<RwLock<AppConfig>>,
        token_manager: &Arc<TokenManager>,
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        bot_id: i64,
        app_id: &str,
        app_secret: &str,
        chats: &[ChatToFetch],
    ) -> Result<(), String> {
        if chats.is_empty() {
            debug!("[feishu-history-fetcher] no chats for bot {}", bot_id);
            return Ok(());
        }

        let feishu_config = FeishuSdkConfig::builder()
            .app_id(app_id)
            .app_secret(app_secret)
            .build();

        let token = token_manager
            .get_tenant_access_token(&feishu_config)
            .await
            .map_err(|e| format!("failed to get token: {}", e))?;

        for chat in chats {
            match Self::fetch_chat_history(
                db,
                executor_registry,
                tx,
                task_manager,
                config,
                token_manager,
                bot_credentials,
                bot_id,
                &chat.chat_id,
                &token,
            ).await {
                Ok(count) => {
                    if count > 0 {
                        info!("[feishu-history-fetcher] fetched {} new messages from chat {}", count, chat.chat_id);
                    }
                }
                Err(e) => {
                    warn!("[feishu-history-fetcher] error fetching chat {}: {}", chat.chat_id, e);
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn fetch_chat_history(
        db: &Arc<Database>,
        executor_registry: &Arc<ExecutorRegistry>,
        tx: &broadcast::Sender<ExecEvent>,
        task_manager: &Arc<TaskManager>,
        _config: &Arc<RwLock<AppConfig>>,
        token_manager: &Arc<TokenManager>,
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        bot_id: i64,
        chat_id: &str,
        token: &str,
    ) -> Result<usize, String> {
        // Get the latest message time from DB for incremental fetching
        let start_time = match db.get_latest_history_message_time(bot_id, chat_id).await {
            Ok(Some(time)) => {
                // Parse the time and convert to Unix timestamp in seconds (Feishu API expects seconds)
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&time) {
                    Some(dt.timestamp().to_string())
                } else {
                    None
                }
            }
            _ => None,
        };

        let mut total_fetched = 0;
        let mut page_token: Option<String> = None;
        let mut has_more = true;

        while has_more {
            let mut query_params: HashMap<String, String> = HashMap::new();
            query_params.insert("container_id_type".to_string(), "chat".to_string());
            query_params.insert("container_id".to_string(), chat_id.to_string());
            query_params.insert("sort_type".to_string(), "ByCreateTimeAsc".to_string());
            query_params.insert("page_size".to_string(), "50".to_string());

            // Use start_time for incremental fetching (messages created after this time)
            if let Some(ref st) = start_time {
                query_params.insert("start_time".to_string(), st.clone());
            }

            if let Some(ref pt) = page_token {
                query_params.insert("page_token".to_string(), pt.clone());
            }

            let resp = Self::list_messages(&reqwest::Client::new(), token, &query_params).await?;

            if resp.code != 0 {
                return Err(format!("API error: {} ({})", resp.msg, resp.code));
            }

            let data = resp.data.ok_or("no data in response")?;
            has_more = data.has_more;
            page_token = data.page_token;

            if let Some(items) = data.items {
                for item in items {
                    if db.feishu_message_exists(&item.message_id).await
                        .map_err(|e| format!("db error: {}", e))?
                    {
                        // Message already exists, skip it but continue fetching
                        // (API returns ascending order, so there may be newer messages)
                        continue;
                    }

                    // Extract sender info from API response
                    let sender_id = item.sender.as_ref().and_then(|s| s.id.clone()).unwrap_or_default();
                    let sender_type = item.sender.as_ref().and_then(|s| s.sender_type.clone());

                    // sender_open_id is actually the sender's ID (open_id for users, app_id for bots)
                    let sender_open_id = sender_id.as_str();

                    let sender_nickname: Option<&str> = None;

                    let content = item.body.as_ref()
                        .and_then(|b| b.content.clone());

                    // create_time from API is in milliseconds, convert to seconds
                    let created_at = item.create_time
                        .and_then(|t| {
                            t.parse::<i64>().ok().map(|ms| ms / 1000)
                        })
                        .and_then(|secs| {
                            chrono::DateTime::from_timestamp(secs, 0)
                        })
                        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                        .unwrap_or_else(|| crate::models::utc_timestamp());

                    if let Err(e) = db.save_feishu_history_message(
                        bot_id,
                        &item.message_id,
                        &item.chat_id,
                        item.chat_type.as_deref().unwrap_or(""),
                        sender_open_id,
                        sender_nickname,
                        sender_type.as_deref(),
                        content.as_deref(),
                        &item.msg_type,
                        &created_at,
                    ).await {
                        warn!("[feishu-history-fetcher] failed to save message {}: {}", item.message_id, e);
                    } else {
                        total_fetched += 1;

                        // Resolve bot's own open_id to filter out self-sent messages
                        let bot_open_id = Self::resolve_bot_open_id(
                            bot_credentials,
                            token_manager,
                            bot_id,
                        ).await;

                        // Process the message through slash command / default response pipeline
                        if let Some(ref msg_content) = content {
                            if let Some((todo_id, execution_record_id)) = Self::process_message(
                                db,
                                executor_registry,
                                tx,
                                task_manager,
                                _config,
                                token_manager,
                                bot_credentials,
                                bot_id,
                                bot_open_id.as_deref(),
                                "group",
                                sender_open_id,
                                chat_id,
                                msg_content,
                            ).await {
                                // Mark message as processed with the triggered todo_id and execution_record_id
                                if let Err(e) = db.mark_feishu_message_processed(&item.message_id, todo_id, execution_record_id).await {
                                    warn!("[feishu-history-fetcher] failed to mark message {} as processed: {}", item.message_id, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(total_fetched)
    }

    /// 处理消息：斜杠命令 -> 默认响应，返回 (todo_id, execution_record_id)
    /// 如果消息发送者是机器人自身，则跳过处理（避免循环触发）
    #[allow(clippy::too_many_arguments)]
    async fn process_message(
        db: &Arc<Database>,
        executor_registry: &Arc<ExecutorRegistry>,
        tx: &broadcast::Sender<ExecEvent>,
        task_manager: &Arc<TaskManager>,
        config: &Arc<RwLock<AppConfig>>,
        token_manager: &Arc<TokenManager>,
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        bot_id: i64,
        bot_open_id: Option<&str>,
        chat_type: &str,
        sender: &str,
        channel: &str,
        content: &str,
    ) -> Option<(i64, Option<i64>)> {
        let trimmed = content.trim();

        // 过滤：如果是机器人自己发送的消息，不处理（防止循环触发）
        // 机器人发送执行结果到群 -> 拉取到历史消息 -> 再次触发执行 -> 循环
        if let Some(bot_id_str) = bot_open_id {
            if sender == bot_id_str {
                tracing::debug!(
                    "[feishu-history] skip self-sent message from bot {}, sender={}",
                    bot_id, sender
                );
                return None;
            }
        }

        // 尝试解析斜杠命令
        if let Some(command_ctx) = Self::parse_slash_command(trimmed) {
            // 有斜杠命令，检查是否匹配规则
            let matched_rule = {
                let cfg = config.read().await;
                cfg.slash_command_rules
                    .iter()
                    .find(|rule| rule.enabled && rule.slash_command.eq_ignore_ascii_case(&command_ctx.command))
                    .cloned()
            };

            if let Some(rule) = matched_rule {
                // 匹配到规则，执行对应的 Todo
                if !command_ctx.body.is_empty() {
                    return Self::execute_slash_command(
                        db,
                        executor_registry,
                        tx,
                        task_manager,
                        config,
                        bot_credentials,
                        token_manager,
                        bot_id,
                        chat_type,
                        sender,
                        channel,
                        command_ctx.command,
                        command_ctx.body,
                        rule.todo_id,
                    ).await;
                }
            }
        }

        // 没有匹配到斜杠命令，检查默认响应
        let default_todo_id = {
            let cfg = config.read().await;
            cfg.default_response_todo_id
        };

        if let Some(todo_id) = default_todo_id {
            if !trimmed.is_empty() {
                return Self::execute_default_response(
                    db,
                    executor_registry,
                    tx,
                    task_manager,
                    bot_credentials,
                    token_manager,
                    bot_id,
                    chat_type,
                    sender,
                    channel,
                    trimmed,
                    todo_id,
                ).await;
            }
        }
        None
    }

    /// 解析斜杠命令
    fn parse_slash_command(content: &str) -> Option<SlashCommandMatch<'_>> {
        let trimmed = content.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let command = parts.next()?.trim();
        let body = parts.next().unwrap_or("").trim();
        Some(SlashCommandMatch { command, body })
    }

    /// 执行斜杠命令，返回 (todo_id, execution_record_id)
    #[allow(clippy::too_many_arguments)]
    async fn execute_slash_command(
        db: &Arc<Database>,
        executor_registry: &Arc<ExecutorRegistry>,
        tx: &broadcast::Sender<ExecEvent>,
        task_manager: &Arc<TaskManager>,
        _config: &Arc<RwLock<AppConfig>>,
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        chat_type: &str,
        sender: &str,
        channel: &str,
        command: &str,
        body: &str,
        todo_id: i64,
    ) -> Option<(i64, Option<i64>)> {
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        let todo = match db.get_todo(todo_id).await {
            Some(todo) => todo,
            None => {
                tracing::warn!("[feishu-history] 斜杠命令绑定的 Todo 不存在: todo_id={}", todo_id);
                return None;
            }
        };

        let mut params = HashMap::new();
        params.insert("content".to_string(), body.to_string());
        params.insert("message".to_string(), body.to_string());
        params.insert("raw_message".to_string(), format!("{} {}", command, body).trim().to_string());
        params.insert("slash_command".to_string(), command.to_string());

        match start_todo_execution(
            db.clone(),
            executor_registry.clone(),
            tx.clone(),
            task_manager.clone(),
            todo.id,
            todo.prompt.clone(),
            todo.executor.clone(),
            "slash_command",
            Some(params),
            None,
            None,
        )
        .await
        {
            Ok(result) => {
                tracing::info!(
                    "[feishu-history] 斜杠命令触发 Todo 执行: command={}, todo_id={}, task_id={}, record_id={:?}",
                    command,
                    todo.id,
                    result.task_id,
                    result.record_id
                );
                // 发送确认消息
                let reply = format!(
                    "已执行命令 {}，正在启动 Todo #{}《{}》。\n任务参数: {}",
                    command,
                    todo.id,
                    todo.title,
                    body
                );
                Self::send_text(bot_credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
                Some((todo.id, result.record_id))
            }
            Err(err) => {
                tracing::error!(
                    "[feishu-history] 斜杠命令执行失败: command={}, todo_id={}, error={:?}",
                    command,
                    todo.id,
                    err
                );
                None
            }
        }
    }

    /// 执行默认响应，返回 (todo_id, execution_record_id)
    #[allow(clippy::too_many_arguments)]
    async fn execute_default_response(
        db: &Arc<Database>,
        executor_registry: &Arc<ExecutorRegistry>,
        tx: &broadcast::Sender<ExecEvent>,
        task_manager: &Arc<TaskManager>,
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        chat_type: &str,
        sender: &str,
        channel: &str,
        body: &str,
        todo_id: i64,
    ) -> Option<(i64, Option<i64>)> {
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        let todo = match db.get_todo(todo_id).await {
            Some(todo) => todo,
            None => {
                tracing::warn!("[feishu-history] 默认响应绑定的 Todo 不存在: todo_id={}", todo_id);
                return None;
            }
        };

        let mut params = HashMap::new();
        params.insert("content".to_string(), body.to_string());
        params.insert("message".to_string(), body.to_string());
        params.insert("raw_message".to_string(), body.to_string());

        match start_todo_execution(
            db.clone(),
            executor_registry.clone(),
            tx.clone(),
            task_manager.clone(),
            todo.id,
            todo.prompt.clone(),
            todo.executor.clone(),
            "slash_command",
            Some(params),
            None,
            None,
        )
        .await
        {
            Ok(result) => {
                tracing::info!(
                    "[feishu-history] 默认响应触发 Todo 执行: todo_id={}, task_id={}, record_id={:?}",
                    todo.id,
                    result.task_id,
                    result.record_id
                );
                let reply = format!(
                    "收到消息，已启动 Todo #{}《{}》。\n任务参数: {}",
                    todo.id,
                    todo.title,
                    body
                );
                Self::send_text(bot_credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
                Some((todo.id, result.record_id))
            }
            Err(err) => {
                tracing::error!(
                    "[feishu-history] 默认响应执行失败: todo_id={}, error={:?}",
                    todo.id,
                    err
                );
                None
            }
        }
    }

    /// 发送文本消息
    async fn send_text(
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        receive_id: &str,
        receive_id_type: &str,
        text: &str,
    ) {
        let base_url = Self::base_url(bot_credentials, bot_id);
        let Some(base_url) = base_url else { return };
        let token = match Self::get_tenant_token(bot_credentials, token_manager, bot_id).await {
            Some(t) => t,
            None => return,
        };

        let client = reqwest::Client::new();
        let url = format!("{}/open-apis/im/v1/messages?receive_id_type={}", base_url, receive_id_type);
        let body = serde_json::json!({
            "receive_id": receive_id,
            "msg_type": "text",
            "content": serde_json::to_string(&serde_json::json!({ "text": text })).unwrap_or_default()
        });

        match client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    tracing::error!("[feishu-history] send_text failed: status={}", status);
                }
            }
            Err(e) => {
                tracing::error!("[feishu-history] send_text request failed: {}", e);
            }
        }
    }

    fn base_url(bot_credentials: &Arc<DashMap<i64, (String, String, String)>>, bot_id: i64) -> Option<String> {
        let domain = bot_credentials.get(&bot_id)?.2.clone();
        Some(if domain == "lark" {
            "https://open.larksuite.com".to_string()
        } else {
            "https://open.feishu.cn".to_string()
        })
    }

    async fn get_tenant_token(
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
    ) -> Option<String> {
        let sdk_config = Self::build_sdk_config(bot_credentials, bot_id)?;
        match token_manager.get_tenant_access_token(&sdk_config).await {
            Ok(token) => Some(token),
            Err(err) => {
                tracing::warn!("[feishu-history] 获取 tenant_access_token 失败: {}", err);
                None
            }
        }
    }

    /// Resolve the bot's own open_id from the Feishu API.
    /// Used to filter out self-sent messages and prevent circular triggering.
    async fn resolve_bot_open_id(
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
    ) -> Option<String> {
        let token = Self::get_tenant_token(bot_credentials, token_manager, bot_id).await?;
        let base_url = Self::base_url(bot_credentials, bot_id)?;

        let client = reqwest::Client::new();
        let res = client
            .get(format!("{base_url}/open-apis/bot/v3/info"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .ok()?;

        let body: serde_json::Value = res.json().await.ok()?;
        body.get("bot")
            .and_then(|b| b.get("open_id"))
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn build_sdk_config(
        bot_credentials: &Arc<DashMap<i64, (String, String, String)>>,
        bot_id: i64,
    ) -> Option<FeishuSdkConfig> {
        let ref_val = bot_credentials.get(&bot_id)?;
        let (app_id, app_secret, domain) = (ref_val.0.clone(), ref_val.1.clone(), ref_val.2.clone());
        let base_url = if domain == "lark" {
            "https://open.larksuite.com"
        } else {
            "https://open.feishu.cn"
        };

        Some(
            FeishuSdkConfig::builder()
                .app_id(app_id)
                .app_secret(app_secret)
                .base_url(base_url)
                .enable_token_cache(true)
                .http_client(reqwest::Client::new())
                .build(),
        )
    }

    async fn list_messages(
        client: &reqwest::Client,
        token: &str,
        query_params: &HashMap<String, String>,
    ) -> Result<ListMessagesResponse, String> {
        let url = format!("https://open.feishu.cn{}", IM_V1_LIST_MESSAGES);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE_JSON));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|e| format!("invalid auth header: {}", e))?,
        );

        let mut builder = client.get(&url).headers(headers);
        for (key, value) in query_params {
            builder = builder.query(&[(key.as_str(), value.as_str())]);
        }

        let resp = builder.send().await
            .map_err(|e| format!("request failed: {}", e))?;

        let status = resp.status();
        let body = resp.text().await
            .map_err(|e| format!("failed to read body: {}", e))?;

        debug!("[feishu-history-fetcher] API response status: {}, body (first 500): {}", status, &body[..body.len().min(500)]);

        let result: ListMessagesResponse = serde_json::from_str(&body)
            .map_err(|e| format!("json parse failed: {}", e))?;

        Ok(result)
    }
}

struct SlashCommandMatch<'a> {
    command: &'a str,
    body: &'a str,
}