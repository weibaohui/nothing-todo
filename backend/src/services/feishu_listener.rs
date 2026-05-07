use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;

use clawrs_feishu::{
    create_channel, Channel, ChannelMessage,
    FeishuConfig, FeishuConnectionMode, FeishuDomain,
    FeishuChannelService,
};

use crate::db::Database;
use crate::models::{AgentBot, BotConfig};

/// Manages WebSocket connections to Feishu for all bound bots.
#[derive(Clone)]
pub struct FeishuListener {
    db: Arc<Database>,
    channels: Arc<DashMap<i64, Arc<FeishuChannelService>>>,
    /// bot_id → (app_id, app_secret, domain)
    bot_credentials: Arc<DashMap<i64, (String, String, String)>>,
}

impl FeishuListener {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
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

        let real_bot_open_id = Self::resolve_bot_open_id(&self.bot_credentials, bot.id).await
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
        tokio::spawn(async move {
            tracing::info!("[feishu:{}] message receiver loop started", bot_id);
            while let Some(msg) = rx.recv().await {
                Self::handle_message(&db, &credentials, bot_id, &bot_open_id, &bot_config_clone, &msg).await;
            }
            tracing::warn!("[feishu:{}] message receiver loop ended", bot_id);
        });

        tracing::info!("feishu listener started for bot {} ({})", bot.id, bot.bot_name);
        Ok(())
    }

    async fn handle_message(
        db: &Database,
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
        let reaction_id = Self::add_reaction(credentials, bot_id, &msg.id, "THUMBSUP").await;

        // /sethome command
        if content == "/sethome" {
            Self::handle_sethome(db, credentials, bot_id, chat_type, &msg.sender, &msg.channel, &msg.id, reaction_id.as_deref()).await;
            return;
        }

        // DM: check dm_enabled
        if chat_type == "p2p" {
            if !bot_config.dm_enabled {
                return;
            }
            if bot_config.echo_reply {
                tracing::info!("[feishu:{}] DM from {}: {}", bot_id, msg.sender, content);
            }
            if let Some(rid) = &reaction_id {
                Self::delete_reaction(credentials, bot_id, &msg.id, rid).await;
            }
            return;
        }

        // Group: check group_enabled and group_require_mention
        // NOTE: 默认只接收 @bot 的群消息。开启飞书权限 im:message.group_msg 后可接收所有群消息。
        if chat_type == "group" {
            if !bot_config.group_enabled {
                return;
            }
            if bot_config.group_require_mention && !is_mention {
                return;
            }
            if bot_config.echo_reply {
                tracing::info!("[feishu:{}] Group {} @mention from {}: {}", bot_id, msg.channel, msg.sender, content);
            }
            if let Some(rid) = &reaction_id {
                Self::delete_reaction(credentials, bot_id, &msg.id, rid).await;
            }
            return;
        }

        if let Some(rid) = &reaction_id {
            Self::delete_reaction(credentials, bot_id, &msg.id, rid).await;
        }
    }

    async fn handle_sethome(
        db: &Database,
        credentials: &DashMap<i64, (String, String, String)>,
        bot_id: i64,
        chat_type: &str,
        sender: &str,
        channel: &str,
        message_id: &str,
        reaction_id: Option<&str>,
    ) {
        let (receive_id, receive_id_type, chat_id) = match chat_type {
            "p2p" => (sender.to_string(), "open_id", None),
            _ => (channel.to_string(), "chat", Some(channel.to_string())),
        };

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

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, bot_id, message_id, rid).await;
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

    // --- Feishu API helpers ---

    fn base_url(credentials: &DashMap<i64, (String, String, String)>, bot_id: i64) -> Option<String> {
        let domain = credentials.get(&bot_id)?.2.clone();
        Some(if domain == "lark" {
            "https://open.larksuite.com".to_string()
        } else {
            "https://open.feishu.cn".to_string()
        })
    }

    async fn get_tenant_token(credentials: &DashMap<i64, (String, String, String)>, bot_id: i64) -> Option<String> {
        let ref_val = credentials.get(&bot_id)?;
        let (app_id, app_secret, domain) = (ref_val.0.clone(), ref_val.1.clone(), ref_val.2.clone());
        let base_url = if domain == "lark" {
            "https://open.larksuite.com"
        } else {
            "https://open.feishu.cn"
        };

        let client = reqwest::Client::new();
        let res = client
            .post(format!("{base_url}/open-apis/auth/v3/tenant_access_token/internal"))
            .json(&serde_json::json!({
                "app_id": app_id.as_str(),
                "app_secret": app_secret.as_str()
            }))
            .send()
            .await
            .ok()?;

        let body: serde_json::Value = res.json().await.ok()?;
        tracing::info!("[feishu:{}] get_tenant_token response code={}", bot_id, body.get("code").and_then(|v| v.as_i64()).unwrap_or(-1));
        body.get("tenant_access_token").and_then(|v| v.as_str()).map(String::from)
    }

    async fn resolve_bot_open_id(
        credentials: &DashMap<i64, (String, String, String)>,
        bot_id: i64,
    ) -> Option<String> {
        let token = Self::get_tenant_token(credentials, bot_id).await?;
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
        bot_id: i64,
        message_id: &str,
        emoji_type: &str,
    ) -> Option<String> {
        let token = Self::get_tenant_token(credentials, bot_id).await?;
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
        bot_id: i64,
        message_id: &str,
        reaction_id: &str,
    ) {
        let token = match Self::get_tenant_token(credentials, bot_id).await {
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
