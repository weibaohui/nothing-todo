use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::db::Database;
use crate::feishu::sdk::config::{Config, CONTENT_TYPE_JSON};

const IM_V1_LIST_MESSAGES: &str = "/open-apis/im/v1/messages";

pub struct FeishuHistoryFetcher {
    db: Arc<Database>,
    client: reqwest::Client,
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
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            client: reqwest::Client::new(),
        }
    }

    pub fn start(&self, bots: Vec<(i64, String, String)>) {
        if bots.is_empty() {
            info!("[feishu-history-fetcher] no bots configured, skipping");
            return;
        }

        let db = self.db.clone();
        let client = self.client.clone();

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

                    if let Err(e) = Self::fetch_for_bot(&db, &client, *bot_id, app_id, app_secret, &bot_chats).await {
                        warn!("[feishu-history-fetcher] error fetching for bot {}: {}", bot_id, e);
                    }
                }
            }
        });
    }

    async fn fetch_for_bot(
        db: &Arc<Database>,
        client: &reqwest::Client,
        bot_id: i64,
        app_id: &str,
        app_secret: &str,
        chats: &[ChatToFetch],
    ) -> Result<(), String> {
        if chats.is_empty() {
            debug!("[feishu-history-fetcher] no chats for bot {}", bot_id);
            return Ok(());
        }

        let config = Config::builder()
            .app_id(app_id)
            .app_secret(app_secret)
            .build();

        let token = config.token_manager.lock().await
            .get_tenant_access_token(&config)
            .await
            .map_err(|e| format!("failed to get token: {}", e))?;

        for chat in chats {
            match Self::fetch_chat_history(db, client, bot_id, &chat.chat_id, &token).await {
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

    async fn fetch_chat_history(
        db: &Arc<Database>,
        client: &reqwest::Client,
        bot_id: i64,
        chat_id: &str,
        token: &str,
    ) -> Result<usize, String> {
        // Get the latest message time from DB for incremental fetching
        let start_time = match db.get_latest_history_message_time(bot_id, chat_id).await {
            Ok(Some(time)) => {
                // Parse the time and convert to Unix timestamp in milliseconds
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&time) {
                    Some(dt.timestamp_millis().to_string())
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

            let resp = Self::list_messages(client, token, &query_params).await?;

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
                    }
                }
            }
        }

        Ok(total_fetched)
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
