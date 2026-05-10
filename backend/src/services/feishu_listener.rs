use std::sync::Arc;
use std::collections::HashMap;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio::sync::{RwLock, broadcast};

use crate::feishu::{
    create_channel, ChannelMessage,
    FeishuConfig, FeishuConnectionMode, FeishuDomain,
    FeishuChannelService,
};
use crate::feishu::sdk::config::Config as FeishuSdkConfig;
use crate::feishu::sdk::token_manager::TokenManager;

use crate::adapters::ExecutorRegistry;
use crate::config::Config as AppConfig;
use crate::db::Database;
use crate::handlers::ExecEvent;
use crate::handlers::execution::start_todo_execution;
use crate::models::{AgentBot, BotConfig};
use crate::task_manager::TaskManager;

/// Manages WebSocket connections to Feishu for all bound bots.
#[derive(Clone)]
pub struct FeishuListener {
    db: Arc<Database>,
    executor_registry: Arc<ExecutorRegistry>,
    tx: broadcast::Sender<ExecEvent>,
    task_manager: Arc<TaskManager>,
    config: Arc<RwLock<AppConfig>>,
    pub token_manager: Arc<TokenManager>,
    channels: Arc<DashMap<i64, Arc<FeishuChannelService>>>,
    /// bot_id → (app_id, app_secret, domain)
    pub bot_credentials: Arc<DashMap<i64, (String, String, String)>>,
}

impl FeishuListener {
    /// 创建飞书监听器。
    pub fn new(
        db: Arc<Database>,
        executor_registry: Arc<ExecutorRegistry>,
        tx: broadcast::Sender<ExecEvent>,
        task_manager: Arc<TaskManager>,
        config: Arc<RwLock<AppConfig>>,
    ) -> Self {
        Self {
            db,
            executor_registry,
            tx,
            task_manager,
            config,
            token_manager: Arc::new(TokenManager::new()),
            channels: Arc::new(DashMap::new()),
            bot_credentials: Arc::new(DashMap::new()),
        }
    }

    pub fn has_bot(&self, bot_id: i64) -> bool {
        self.channels.contains_key(&bot_id)
    }

    pub async fn start_bot(&self, bot: &AgentBot) -> anyhow::Result<()> {
        let domain = match bot.domain.as_deref() {
            Some("lark") => FeishuDomain::Lark,
            _ => FeishuDomain::Feishu,
        };

        let bot_config: BotConfig = serde_json::from_str(&bot.config).unwrap_or_default();

        let config = FeishuConfig {
            app_id: bot.app_id.clone().into(),
            app_secret: bot.app_secret.clone().into(),
            domain: domain.clone(),
            connection_mode: FeishuConnectionMode::WebSocket,
            allowed_users: vec!["*".into()],
            group_require_mention: bot_config.group_require_mention,
            dm_policy: None,
            group_policy: None,
            allow_from: None,
            group_allow_from: vec![],
            encrypt_key: None,
            verification_token: None,
            webhook_port: None,
        };

        let channel = Arc::new(create_channel(config));
        let (tx, mut rx) = mpsc::channel::<ChannelMessage>(256);

        let ch = channel.clone();
        let bot_id = bot.id;
        tokio::spawn(async move {
            tracing::info!("[feishu:{}] starting listen()", bot_id);
            match ch.listen(tx).await {
                Ok(()) => tracing::warn!("[feishu:{}] listen() returned Ok", bot_id),
                Err(e) => tracing::error!("[feishu:{}] listen() error: {e}", bot_id),
            }
        });

        self.channels.insert(bot.id, channel);
        let domain_str = match domain {
            FeishuDomain::Lark => "lark",
            _ => "feishu",
        };
        self.bot_credentials.insert(bot.id, (bot.app_id.clone(), bot.app_secret.clone(), domain_str.to_string()));

        let real_bot_open_id = Self::resolve_bot_open_id(&self.bot_credentials, &self.token_manager, bot.id).await
            .or(bot.bot_open_id.clone())
            .unwrap_or_default();
        if real_bot_open_id != bot.bot_open_id.clone().unwrap_or_default() {
            tracing::info!(
                "[feishu:{}] corrected bot_open_id from {:?} to {}",
                bot.id, bot.bot_open_id, real_bot_open_id
            );
        }

        let db = self.db.clone();
        let bot_open_id = real_bot_open_id;
        let bot_config_clone = bot_config;
        let credentials = self.bot_credentials.clone();
        let executor_registry = self.executor_registry.clone();
        let tx = self.tx.clone();
        let task_manager = self.task_manager.clone();
        let config = self.config.clone();
        let token_manager = self.token_manager.clone();
        tokio::spawn(async move {
            tracing::info!("[feishu:{}] message receiver loop started", bot_id);
            while let Some(msg) = rx.recv().await {
                Self::handle_message(
                    &db,
                    &executor_registry,
                    &tx,
                    &task_manager,
                    &config,
                    &token_manager,
                    &credentials,
                    bot_id,
                    &bot_open_id,
                    &bot_config_clone,
                    &msg,
                )
                .await;
            }
            tracing::warn!("[feishu:{}] message receiver loop ended", bot_id);
        });

        tracing::info!("feishu listener started for bot {} ({})", bot.id, bot.bot_name);
        Ok(())
    }

    async fn handle_message(
        db: &Arc<Database>,
        executor_registry: &Arc<ExecutorRegistry>,
        tx: &broadcast::Sender<ExecEvent>,
        task_manager: &Arc<TaskManager>,
        config: &Arc<RwLock<AppConfig>>,
        token_manager: &Arc<TokenManager>,
        credentials: &DashMap<i64, (String, String, String)>,
        bot_id: i64,
        bot_open_id: &str,
        bot_config: &BotConfig,
        msg: &ChannelMessage,
    ) {
        tracing::info!(
            "[feishu:{}] handle_message: sender={}, bot_open_id={}, content={:?}, chat_type={:?}",
            bot_id, msg.sender, bot_open_id, msg.content, msg.chat_type
        );

        if msg.sender == bot_open_id {
            tracing::info!("[feishu:{}] skipping self-sent message", bot_id);
            return;
        }

        let chat_type = msg.chat_type.as_deref().unwrap_or("p2p");
        let is_mention = !msg.mentioned_open_ids.is_empty();

        db.save_feishu_message(
            bot_id,
            &msg.id,
            &msg.channel,
            chat_type,
            &msg.sender,
            Some(&msg.content),
            "text",
            is_mention,
        )
        .await
        .ok();

        let content = msg.content.trim();

        // Add "processing" reaction
        let reaction_id = Self::add_reaction(credentials, token_manager, bot_id, &msg.id, "THUMBSUP").await;

        // /sethome command
        if content == "/sethome" {
            Self::handle_sethome(db, credentials, token_manager, bot_id, chat_type, &msg.sender, &msg.channel, &msg.id, reaction_id.as_deref()).await;
            return;
        }

        // /feishupush command (toggle push)
        if content == "/feishupush" {
            Self::handle_feishupush(db, credentials, token_manager, bot_id, chat_type, &msg.sender, &msg.channel, &msg.id, reaction_id.as_deref()).await;
            return;
        }

        if !Self::is_message_allowed(chat_type, is_mention, bot_config) {
            if let Some(rid) = &reaction_id {
                Self::delete_reaction(credentials, token_manager, bot_id, &msg.id, rid).await;
            }
            return;
        }

        // Check if message response is enabled for this chat type
        let response_enabled = db.get_feishu_response_enabled(bot_id, chat_type).await.unwrap_or(false);

        if !response_enabled {
            tracing::info!("[feishu:{}] message response is disabled for {} chat type", bot_id, chat_type);
            if let Some(rid) = &reaction_id {
                Self::delete_reaction(credentials, token_manager, bot_id, &msg.id, rid).await;
            }
            return;
        }

        if let Some(command_ctx) = Self::parse_slash_command(content) {
            let triggered = Self::handle_custom_slash_command(
                db,
                executor_registry,
                tx,
                task_manager,
                config,
                credentials,
                token_manager,
                bot_id,
                chat_type,
                &msg.sender,
                &msg.channel,
                command_ctx.command,
                command_ctx.body,
            )
            .await;
            if triggered.is_some() {
                if let Some(rid) = &reaction_id {
                    Self::delete_reaction(credentials, token_manager, bot_id, &msg.id, rid).await;
                }
                // 标记消息为已处理
                if let Some((todo_id, execution_record_id)) = triggered {
                    if let Err(e) = db.mark_feishu_message_processed(&msg.id, todo_id, execution_record_id).await {
                        tracing::warn!("[feishu:{}] failed to mark message {} as processed: {}", bot_id, msg.id, e);
                    }
                }
                return;
            }
        } else {
            // 非斜杠命令消息，也检查是否需要默认响应
            let default_todo_id = {
                let cfg = config.read().await;
                cfg.default_response_todo_id
            };

            if let Some(todo_id) = default_todo_id {
                if !content.is_empty() {
                    let triggered = Self::execute_default_response(
                        db,
                        executor_registry,
                        tx,
                        task_manager,
                        config,
                        credentials,
                        token_manager,
                        bot_id,
                        chat_type,
                        &msg.sender,
                        &msg.channel,
                        "",
                        content,
                        todo_id,
                    )
                    .await;
                    if let Some(rid) = &reaction_id {
                        Self::delete_reaction(credentials, token_manager, bot_id, &msg.id, rid).await;
                    }
                    // 标记消息为已处理，使用实际触发的 todo_id 和 execution_record_id
                    if let Some((actual_todo_id, execution_record_id)) = triggered {
                        if let Err(e) = db.mark_feishu_message_processed(&msg.id, actual_todo_id, execution_record_id).await {
                            tracing::warn!("[feishu:{}] failed to mark message {} as processed: {}", bot_id, msg.id, e);
                        }
                    }
                    return;
                }
            }
        }

        if chat_type == "p2p" && bot_config.echo_reply {
            tracing::info!("[feishu:{}] 收到私聊消息: sender={}, content={}", bot_id, msg.sender, content);
        }
        if chat_type == "group" && bot_config.echo_reply {
            tracing::info!("[feishu:{}] 收到群聊消息: channel={}, sender={}, content={}", bot_id, msg.channel, msg.sender, content);
        }

        if let Some(rid) = &reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, &msg.id, rid).await;
        }
    }

    /// 判断当前消息是否符合接收配置。
    fn is_message_allowed(chat_type: &str, is_mention: bool, bot_config: &BotConfig) -> bool {
        match chat_type {
            "p2p" => bot_config.dm_enabled,
            "group" => {
                if !bot_config.group_enabled {
                    return false;
                }
                if bot_config.group_require_mention && !is_mention {
                    return false;
                }
                true
            }
            _ => true,
        }
    }

    /// 解析斜杠命令，只匹配首个词。
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

    /// 处理用户自定义斜杠命令，并按配置路由到指定 Todo。返回 (todo_id, execution_record_id)。
    #[allow(clippy::too_many_arguments)]
    async fn handle_custom_slash_command(
        db: &Arc<Database>,
        executor_registry: &Arc<ExecutorRegistry>,
        tx: &broadcast::Sender<ExecEvent>,
        task_manager: &Arc<TaskManager>,
        config: &Arc<RwLock<AppConfig>>,
        credentials: &DashMap<i64, (String, String, String)>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        chat_type: &str,
        sender: &str,
        channel: &str,
        command: &str,
        body: &str,
    ) -> Option<(i64, Option<i64>)> {
        let cmd_lc = command.to_ascii_lowercase();
        let matched_rule = {
            let cfg = config.read().await;
            cfg.slash_command_rules
                .iter()
                .find(|rule| rule.enabled && rule.slash_command.eq_ignore_ascii_case(&cmd_lc))
                .cloned()
        };

        // 如果没有匹配到斜杠命令规则，检查是否需要默认响应
        if matched_rule.is_none() {
            let default_todo_id = {
                let cfg = config.read().await;
                cfg.default_response_todo_id
            };

            if let Some(todo_id) = default_todo_id {
                if !body.is_empty() {
                    // 执行默认响应的 Todo
                    return Self::execute_default_response(
                        db,
                        executor_registry,
                        tx,
                        task_manager,
                        config,
                        credentials,
                        token_manager,
                        bot_id,
                        chat_type,
                        sender,
                        channel,
                        command,
                        body,
                        todo_id,
                    ).await;
                }
            }
            return None;
        }

        let rule = matched_rule.unwrap();

        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        if body.is_empty() {
            let reply = format!("命令 {} 需要输入内容，例如：{} 帮我整理今天的任务", command, command);
            Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
            return None;
        }

        let todo = match db.get_todo(rule.todo_id).await {
            Some(todo) => todo,
            None => {
                let reply = format!("命令 {} 绑定的 Todo #{} 不存在，请到设置中重新配置。", command, rule.todo_id);
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
                tracing::warn!("[feishu:{}] 斜杠命令绑定的 Todo 不存在: command={}, todo_id={}", bot_id, command, rule.todo_id);
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
                let reply = format!(
                    "已执行命令 {}，正在启动 Todo #{}《{}》。\n任务参数: {}",
                    command,
                    todo.id,
                    todo.title,
                    body
                );
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
                tracing::info!(
                    "[feishu:{}] 斜杠命令触发 Todo 执行: command={}, todo_id={}, task_id={}, record_id={:?}",
                    bot_id,
                    command,
                    todo.id,
                    result.task_id,
                    result.record_id
                );
                return Some((todo.id, result.record_id));
            }
            Err(err) => {
                let reply = format!("命令 {} 执行失败: {}", command, Self::error_message(&err));
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
                tracing::error!(
                    "[feishu:{}] 斜杠命令执行失败: command={}, todo_id={}, error={}",
                    bot_id,
                    command,
                    todo.id,
                    Self::error_message(&err)
                );
                return None;
            }
        }
    }

    /// 执行默认响应：当没有匹配到斜杠命令时，执行指定的 Todo。返回 (todo_id, execution_record_id)。
    #[allow(clippy::too_many_arguments)]
    async fn execute_default_response(
        db: &Arc<Database>,
        executor_registry: &Arc<ExecutorRegistry>,
        tx: &broadcast::Sender<ExecEvent>,
        task_manager: &Arc<TaskManager>,
        _config: &Arc<RwLock<AppConfig>>,
        credentials: &DashMap<i64, (String, String, String)>,
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
                let reply = format!("默认响应绑定的 Todo #{} 不存在，请到设置中重新配置。", todo_id);
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
                tracing::warn!("[feishu:{}] 默认响应绑定的 Todo 不存在: todo_id={}", bot_id, todo_id);
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
                let reply = format!(
                    "收到消息，已启动 Todo #{}《{}》。\n任务参数: {}",
                    todo.id,
                    todo.title,
                    body
                );
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
                tracing::info!(
                    "[feishu:{}] 默认响应触发 Todo 执行: todo_id={}, task_id={}, record_id={:?}",
                    bot_id,
                    todo.id,
                    result.task_id,
                    result.record_id
                );
                Some((todo.id, result.record_id))
            }
            Err(err) => {
                let reply = format!("执行失败: {}", Self::error_message(&err));
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &reply).await;
                tracing::error!(
                    "[feishu:{}] 默认响应执行失败: todo_id={}, error={}",
                    bot_id,
                    todo.id,
                    Self::error_message(&err)
                );
                None
            }
        }
    }

    /// 将应用错误转换为可回复的文本。
    fn error_message(err: &crate::handlers::AppError) -> String {
        match err {
            crate::handlers::AppError::NotFound => "未找到相关资源".to_string(),
            crate::handlers::AppError::BadRequest(msg) => msg.clone(),
            crate::handlers::AppError::Internal(msg) => msg.clone(),
        }
    }

    async fn handle_sethome(
        db: &Database,
        credentials: &DashMap<i64, (String, String, String)>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        chat_type: &str,
        sender: &str,
        channel: &str,
        message_id: &str,
        reaction_id: Option<&str>,
    ) {
        let target_type = if chat_type == "p2p" { "p2p" } else { "group" };
        let (receive_id, receive_id_type, chat_id) = match chat_type {
            "p2p" => (sender.to_string(), "open_id", None),
            _ => (channel.to_string(), "chat_id", Some(channel.to_string())),
        };

        // Update feishu_home
        match db
            .set_feishu_home(bot_id, sender, chat_id.as_deref(), &receive_id, receive_id_type)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "[feishu:{}] /sethome by {} → {} ({})",
                    bot_id, sender, receive_id, receive_id_type
                );
            }
            Err(e) => {
                tracing::error!("[feishu:{}] /sethome failed: {e}", bot_id);
            }
        }

        // Update only the relevant push target field
        if chat_type == "p2p" {
            if let Err(e) = db.set_p2p_receive_id(bot_id, &receive_id).await {
                tracing::error!("[feishu:{}] set p2p push target failed: {e}", bot_id);
            }
        } else {
            if let Err(e) = db.set_group_chat_id(bot_id, channel).await {
                tracing::error!("[feishu:{}] set group push target failed: {e}", bot_id);
            }
        }

        // Enable message response for this chat type
        if let Err(e) = db
            .set_feishu_response_enabled(bot_id, target_type, true)
            .await
        {
            tracing::error!("[feishu:{}] enable response failed: {e}", bot_id);
        }

        // Send confirmation
        let chat_type_label = if chat_type == "p2p" { "私聊" } else { "群聊" };
        let target_desc = if chat_type == "p2p" { "此私聊" } else { channel };
        let confirm = format!("✅ 已设置推送目标为此 {chat_type_label} ({target_desc})，执行过程将实时推送。\n\n如需关闭推送，请发送 /feishupush");
        Self::send_text(credentials, token_manager, bot_id, &receive_id, &receive_id_type, &confirm).await;

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
        }
    }

    /// Handle /feishupush - cycle push level: disabled -> result_only -> all -> disabled.
    async fn handle_feishupush(
        db: &Database,
        credentials: &DashMap<i64, (String, String, String)>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        chat_type: &str,
        sender: &str,
        channel: &str,
        message_id: &str,
        reaction_id: Option<&str>,
    ) {
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        let target = db.get_feishu_push_target(bot_id).await.ok().flatten();
        let current_level = target.as_ref().map(|t| t.push_level.as_str()).unwrap_or("disabled");
        let new_level = match current_level {
            "disabled" => "result_only",
            "result_only" => "all",
            "all" => "disabled",
            _ => "disabled",
        };

        if let Err(e) = db.update_feishu_push_level(bot_id, new_level).await {
            tracing::error!("[feishu:{}] /feishupush update failed: {e}", bot_id);
            let msg = "⚠️ 操作失败，请稍后重试";
            Self::send_text(credentials, token_manager, bot_id, &receive_id, &receive_id_type, msg).await;
        } else {
            let (status_text, status_emoji) = match new_level {
                "disabled" => ("关闭", "ℹ️"),
                "result_only" => ("已切换为仅结论", "✅"),
                "all" => ("已切换为全部", "✅"),
                _ => ("未知", "⚠️"),
            };
            let msg = format!("{} 推送{}。", status_emoji, status_text);
            Self::send_text(credentials, token_manager, bot_id, &receive_id, &receive_id_type, &msg).await;
            tracing::info!("[feishu:{}] /feishupush: push level changed to {} for bot_id={}", bot_id, new_level, bot_id);
        }

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
        }
    }

    /// Send a plain text message to a Feishu recipient.
    async fn send_text(
        credentials: &DashMap<i64, (String, String, String)>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        receive_id: &str,
        receive_id_type: &str,
        text: &str,
    ) {
        let base_url = match Self::base_url(credentials, bot_id) {
            Some(u) => u,
            None => return,
        };
        let token = match Self::get_tenant_token(credentials, token_manager, bot_id).await {
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
                    tracing::error!("[feishu:{}] send_text failed: status={}", bot_id, status);
                } else {
                    tracing::debug!("[feishu:{}] send_text ok to {} ({})", bot_id, receive_id, receive_id_type);
                }
            }
            Err(e) => {
                tracing::error!("[feishu:{}] send_text request failed: {e}", bot_id);
            }
        }
    }

    /// Send a message via a specific bot's channel.
    pub async fn send(&self, bot_id: i64, text: &str, recipient: &str) -> anyhow::Result<()> {
        if let Some(ch) = self.channels.get(&bot_id) {
            ch.send(text, recipient).await?;
            Ok(())
        } else {
            anyhow::bail!("bot {} not running", bot_id)
        }
    }

    /// Send a raw text message using a specific receive_id_type.
    pub async fn send_raw(
        &self,
        bot_id: i64,
        receive_id: &str,
        receive_id_type: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let base_url = Self::base_url(&self.bot_credentials, bot_id)
            .ok_or_else(|| anyhow::anyhow!("no credentials for bot {}", bot_id))?;
        let token = Self::get_tenant_token(&self.bot_credentials, &self.token_manager, bot_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("no token for bot {}", bot_id))?;

        let client = reqwest::Client::new();
        let url = format!("{}/open-apis/im/v1/messages?receive_id_type={}", base_url, receive_id_type);
        let body = serde_json::json!({
            "receive_id": receive_id,
            "msg_type": "text",
            "content": serde_json::to_string(&serde_json::json!({ "text": text })).unwrap_or_default()
        });

        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            let body: serde_json::Value = res.json().await.unwrap_or_default();
            return Err(anyhow::anyhow!("send_raw failed: {} {:?}", status, body));
        }

        Ok(())
    }

    // --- Feishu API helpers ---

    fn base_url(credentials: &DashMap<i64, (String, String, String)>, bot_id: i64) -> Option<String> {
        let domain = credentials.get(&bot_id)?.2.clone();
        Some(if domain == "lark" {
            "https://open.larksuite.com".to_string()
        } else {
            "https://open.feishu.cn".to_string()
        })
    }

    fn build_sdk_config(
        credentials: &DashMap<i64, (String, String, String)>,
        bot_id: i64,
    ) -> Option<FeishuSdkConfig> {
        let ref_val = credentials.get(&bot_id)?;
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

    async fn get_tenant_token(
        credentials: &DashMap<i64, (String, String, String)>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
    ) -> Option<String> {
        let sdk_config = Self::build_sdk_config(credentials, bot_id)?;
        match token_manager.get_tenant_access_token(&sdk_config).await {
            Ok(token) => Some(token),
            Err(err) => {
                tracing::warn!("[feishu:{}] 获取 tenant_access_token 失败: {}", bot_id, err);
                None
            }
        }
    }

    async fn resolve_bot_open_id(
        credentials: &DashMap<i64, (String, String, String)>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
    ) -> Option<String> {
        let token = Self::get_tenant_token(credentials, token_manager, bot_id).await?;
        let base_url = Self::base_url(credentials, bot_id)?;

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

    /// Add reaction, returns reaction_id on success.
    async fn add_reaction(
        credentials: &DashMap<i64, (String, String, String)>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        message_id: &str,
        emoji_type: &str,
    ) -> Option<String> {
        let token = Self::get_tenant_token(credentials, token_manager, bot_id).await?;
        let base_url = Self::base_url(credentials, bot_id)?;

        let client = reqwest::Client::new();
        let url = format!("{base_url}/open-apis/im/v1/messages/{message_id}/reactions");
        let body_json = serde_json::json!({
            "reaction_type": {
                "emoji_type": emoji_type
            }
        });
        tracing::info!("[feishu:{}] add_reaction POST {} token={}... body={}", bot_id, url, &token[..token.len().min(10)], body_json);
        let res = match client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body_json)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("[feishu:{}] add_reaction request failed: {e}", bot_id);
                return None;
            }
        };

        let status = res.status();
        let body: serde_json::Value = match res.json().await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("[feishu:{}] add_reaction parse failed: {e}", bot_id);
                return None;
            }
        };

        let code = body.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            tracing::error!("[feishu:{}] add_reaction API error (status={}): {body}", bot_id, status);
            return None;
        }

        let reaction_id = body
            .get("data")
            .and_then(|d| d.get("reaction_id"))
            .and_then(|v| v.as_str())
            .map(String::from);

        tracing::info!("[feishu:{}] add_reaction {} ok, reaction_id={:?}", bot_id, emoji_type, reaction_id);
        reaction_id
    }

    /// Delete reaction by reaction_id.
    async fn delete_reaction(
        credentials: &DashMap<i64, (String, String, String)>,
        token_manager: &Arc<TokenManager>,
        bot_id: i64,
        message_id: &str,
        reaction_id: &str,
    ) {
        let token = match Self::get_tenant_token(credentials, token_manager, bot_id).await {
            Some(t) => t,
            None => return,
        };
        let base_url = match Self::base_url(credentials, bot_id) {
            Some(u) => u,
            None => return,
        };

        let client = reqwest::Client::new();
        match client
            .delete(format!("{base_url}/open-apis/im/v1/messages/{message_id}/reactions/{reaction_id}"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
        {
            Ok(res) => {
                let body: serde_json::Value = res.json().await.unwrap_or_default();
                let code = body.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
                if code == 0 {
                    tracing::info!("[feishu:{}] delete_reaction ok", bot_id);
                } else {
                    tracing::error!("[feishu:{}] delete_reaction API error: {body}", bot_id);
                }
            }
            Err(e) => {
                tracing::error!("[feishu:{}] delete_reaction request failed: {e}", bot_id);
            }
        }
    }
}

struct SlashCommandMatch<'a> {
    command: &'a str,
    body: &'a str,
}

#[cfg(test)]
mod tests {
    use super::FeishuListener;
    use crate::models::BotConfig;

    #[test]
    fn test_parse_slash_command_exact_first_token() {
        let parsed = FeishuListener::parse_slash_command("/todo 帮我整理今天任务").unwrap();
        assert_eq!(parsed.command, "/todo");
        assert_eq!(parsed.body, "帮我整理今天任务");
    }

    #[test]
    fn test_parse_slash_command_without_body() {
        let parsed = FeishuListener::parse_slash_command("/todo").unwrap();
        assert_eq!(parsed.command, "/todo");
        assert_eq!(parsed.body, "");
    }

    #[test]
    fn test_group_message_requires_mention_when_enabled() {
        let cfg = BotConfig {
            group_enabled: true,
            group_require_mention: true,
            ..Default::default()
        };
        assert!(!FeishuListener::is_message_allowed("group", false, &cfg));
        assert!(FeishuListener::is_message_allowed("group", true, &cfg));
    }
}
