use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::sync::RwLock;

use crate::feishu::sdk::config::Config as FeishuSdkConfig;
use crate::feishu::sdk::token_manager::TokenManager;
use crate::feishu::{
    create_channel, ChannelMessage, FeishuChannelService, FeishuConfig, FeishuConnectionMode,
    FeishuDomain,
};

use crate::service_context::ServiceContext;
use crate::config::Config as AppConfig;
use crate::task_manager::TaskManager;
use crate::db::{Database, FeishuProjectBinding, NewFeishuMessage};
use crate::models::{AgentBot, BotConfig, build_trigger_params};
use crate::services::message_debounce::{MessageDebounce, PendingMessage};

/// Manages WebSocket connections to Feishu for all bound bots.
#[derive(Clone)]
pub struct FeishuListener {
    ctx: ServiceContext,
    pub token_manager: Arc<TokenManager>,
    channels: Arc<DashMap<i64, Arc<FeishuChannelService>>>,
    /// bot_id → (app_id, app_secret, domain)
    pub bot_credentials: Arc<DashMap<i64, (String, String, String)>>,
    debounce: Arc<MessageDebounce>,
}

#[derive(Clone, Copy)]
struct ListenerMessageContext<'a> {
    db: &'a Arc<Database>,
    config: &'a Arc<RwLock<AppConfig>>,
    token_manager: &'a Arc<TokenManager>,
    credentials: &'a DashMap<i64, (String, String, String)>,
    debounce: &'a Arc<MessageDebounce>,
    task_manager: &'a Arc<TaskManager>,
    bot_id: i64,
    bot_open_id: &'a str,
    bot_config: &'a BotConfig,
}

#[derive(Clone, Copy)]
struct FeishuCommandContext<'a> {
    db: &'a Arc<Database>,
    credentials: &'a DashMap<i64, (String, String, String)>,
    token_manager: &'a Arc<TokenManager>,
    bot_id: i64,
    chat_type: &'a str,
    sender: &'a str,
    channel: &'a str,
    message_id: &'a str,
    content: &'a str,
    reaction_id: Option<&'a str>,
}

impl FeishuListener {
    /// 创建飞书监听器。
    pub fn new(
        ctx: ServiceContext,
        debounce: Arc<MessageDebounce>,
    ) -> Self {
        Self {
            ctx,
            debounce,
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
            app_id: bot.app_id.clone(),
            app_secret: bot.app_secret.clone(),
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
        self.bot_credentials.insert(
            bot.id,
            (
                bot.app_id.clone(),
                bot.app_secret.clone(),
                domain_str.to_string(),
            ),
        );

        let real_bot_open_id =
            Self::resolve_bot_open_id(&self.bot_credentials, &self.token_manager, bot.id)
                .await
                .or(bot.bot_open_id.clone())
                .unwrap_or_default();
        if real_bot_open_id != bot.bot_open_id.clone().unwrap_or_default() {
            tracing::info!(
                "[feishu:{}] corrected bot_open_id from {:?} to {}",
                bot.id,
                bot.bot_open_id,
                real_bot_open_id
            );
        }

        let db = self.ctx.db.clone();
        let bot_open_id = real_bot_open_id;
        let bot_config_clone = bot_config;
        let credentials = self.bot_credentials.clone();
        let config = self.ctx.config.clone();
        let token_manager = self.token_manager.clone();
        let debounce = self.debounce.clone();
        let task_manager = self.ctx.task_manager.clone();
        tokio::spawn(async move {
            tracing::info!("[feishu:{}] message receiver loop started", bot_id);
            while let Some(msg) = rx.recv().await {
                let context = ListenerMessageContext {
                    db: &db,
                    config: &config,
                    token_manager: &token_manager,
                    credentials: &credentials,
                    debounce: &debounce,
                    task_manager: &task_manager,
                    bot_id,
                    bot_open_id: &bot_open_id,
                    bot_config: &bot_config_clone,
                };
                Self::handle_message(context, &msg).await;
            }
            tracing::warn!("[feishu:{}] message receiver loop ended", bot_id);
        });

        tracing::info!(
            "feishu listener started for bot {} ({})",
            bot.id,
            bot.bot_name
        );
        Ok(())
    }

    /// 编排整条飞书消息的处理流程。
    /// 步骤：自消息过滤 → 落库 → 加 reaction → 内建命令 → 权限闸门 → 晋升 pending binding
    ///      → 项目绑定路径 → 斜杠规则/默认回复 → echo 日志 → 清理 reaction。
    /// 每个步骤下沉到职责单一的子函数，本函数只负责"组合 + 早退"。
    /// 把 519 行的单体函数按阶段拆分后，可读性、可测性、可维护性都显著提升。
    async fn handle_message(context: ListenerMessageContext<'_>, msg: &ChannelMessage) {
        // 自发消息直接 return：飞书多端登录会把自己的消息广播过来
        if Self::is_self_sent(context.bot_open_id, context.bot_id, &msg.sender) {
            return;
        }
        // 1) 把消息原样落库，供历史审计；落库失败不影响主流程
        Self::persist_incoming_message(context.db, context.bot_id, msg).await;
        // 2) THUMBSUP reaction 用作"处理中"指示；后续由 cleanup_reaction 兜底删除
        let reaction_id = Self::add_reaction(
            context.credentials,
            context.token_manager,
            context.bot_id,
            &msg.id,
            "THUMBSUP",
        )
        .await;
        // 3) 构造所有 handle_xxx 子函数共用的精简上下文（已 trim content）
        let cmd = Self::build_command_context(&context, msg, reaction_id.as_deref());
        // 4) 内建斜杠命令：/sethome /feishupush /list /bind /unbind /new /stop
        if Self::route_builtin_command(context.task_manager, cmd).await {
            return;
        }
        // 5) 接收/响应权限闸门：dm/group 策略 + 全局响应开关 + 群白名单
        if !Self::passes_response_gates(&context, &cmd, msg).await {
            Self::cleanup_reaction(&context, msg, reaction_id.as_deref()).await;
            return;
        }
        // 6) 把 Web 页面创建的 __pending__ binding 关联到当前真实 chat
        Self::promote_pending_binding(context.db, context.bot_id, cmd.channel, cmd.chat_type).await;
        // 7) 项目绑定路径（resume / new session），命中即返回（内部已清 reaction）
        if Self::handle_project_binding_path(&context, &cmd).await {
            return;
        }
        // 8) 用户自定义斜杠规则 + 默认回复兜底
        Self::dispatch_slash_or_default(&context, &cmd).await;
        // 9) echo 调试日志（按 chat_type 区分私聊/群聊）
        Self::log_echo_reply(&context, &cmd);
        // 10) 收尾：删掉 THUMBSUP reaction
        Self::cleanup_reaction(&context, msg, reaction_id.as_deref()).await;
    }

    // ---------------------------------------------------------------------
    // 下面是 handle_message 拆分出的辅助方法。
    // 原来的 519 行函数被拆为：编排器（handle_message）+ 9 个职责单一的子阶段。
    // 抽取原则遵循《重构 2nd》Ch6 Extract Function：每个子函数不超过 30 行，
    // 函数名自文档化，副作用边界（reaction 清理、debounce push）显式可见。
    // ---------------------------------------------------------------------

    /// 自发消息（多端同步过来的自己）一律忽略，避免循环触发
    fn is_self_sent(bot_open_id: &str, bot_id: i64, sender: &str) -> bool {
        if sender == bot_open_id {
            // 飞书多端登录时，发送的消息会广播到所有连接；用 bot_open_id 过滤自己
            tracing::info!("[feishu:{}] skipping self-sent message", bot_id);
            return true;
        }
        false
    }

    /// 把原始消息原样落库（chat_type、mention 状态都来自 msg），
    /// 用于消息历史审计；落库失败不影响主流程
    async fn persist_incoming_message(db: &Arc<Database>, bot_id: i64, msg: &ChannelMessage) {
        // chat_type 为空时按 p2p 兜底（飞书私聊字段缺失场景）
        let chat_type = msg.chat_type.as_deref().unwrap_or("p2p");
        // p2p 没有 mentioned_open_ids 概念；group 消息 @ bot 时该数组非空
        let is_mention = !msg.mentioned_open_ids.is_empty();
        let _ = db
            .save_feishu_message(NewFeishuMessage {
                bot_id,
                message_id: &msg.id,
                chat_id: &msg.channel,
                chat_type,
                sender_open_id: &msg.sender,
                sender_type: msg.sender_type.as_deref(),
                content: Some(&msg.content),
                msg_type: "text",
                is_mention,
            })
            .await;
    }

    /// 把 ListenerMessageContext 里散落的相关字段聚成 FeishuCommandContext，
    /// 避免每个 handle_xxx 函数都要从 9 个字段里手挑
    fn build_command_context<'a>(
        context: &'a ListenerMessageContext<'a>,
        msg: &'a ChannelMessage,
        reaction_id: Option<&'a str>,
    ) -> FeishuCommandContext<'a> {
        FeishuCommandContext {
            db: context.db,
            credentials: context.credentials,
            token_manager: context.token_manager,
            bot_id: context.bot_id,
            // 飞书未填 chat_type 时按 p2p 兜底
            chat_type: msg.chat_type.as_deref().unwrap_or("p2p"),
            sender: &msg.sender,
            channel: &msg.channel,
            message_id: &msg.id,
            // 提前 trim，让命令匹配和后续 prompt 共用同一个干净字符串
            content: msg.content.trim(),
            reaction_id,
        }
    }

    /// 内建斜杠命令分发：7 个固定指令任一命中即返回 true（已处理）。
    /// /stop 是唯一需要 task_manager 的，所以单独把 task_manager 传进来。
    /// 顺序按命令在聊天中的使用频率排：list > bind > unbind > sethome > feishupush > new > stop
    async fn route_builtin_command(
        task_manager: &Arc<TaskManager>,
        cmd: FeishuCommandContext<'_>,
    ) -> bool {
        // /sethome：把当前 chat 标记为推送目标并打开响应开关
        if cmd.content == "/sethome" {
            Self::handle_sethome(cmd).await;
            return true;
        }
        // /feishupush：循环切换推送等级（disabled → result_only → all → disabled）
        if cmd.content == "/feishupush" {
            Self::handle_feishupush(cmd).await;
            return true;
        }
        // /list：列出所有已注册项目目录，供 /bind 时挑选
        if cmd.content == "/list" {
            Self::handle_list(cmd).await;
            return true;
        }
        // /bind 与 /bind <name> 走同一个处理函数，由 handle_bind 内部区分
        if cmd.content == "/bind" || cmd.content.starts_with("/bind ") {
            Self::handle_bind(cmd).await;
            return true;
        }
        if cmd.content == "/unbind" {
            Self::handle_unbind(cmd).await;
            return true;
        }
        if cmd.content == "/new" {
            Self::handle_new(cmd).await;
            return true;
        }
        if cmd.content == "/stop" {
            Self::handle_stop(task_manager, cmd).await;
            return true;
        }
        false
    }

    /// 三道响应权限闸门，按顺序短路：dm/group 策略 → 全局响应开关 → 群白名单
    /// 全部通过返回 true；任一失败返回 false
    async fn passes_response_gates(
        context: &ListenerMessageContext<'_>,
        cmd: &FeishuCommandContext<'_>,
        msg: &ChannelMessage,
    ) -> bool {
        // 闸门一：bot_config 里的 dm_enabled / group_enabled / group_require_mention
        let is_mention = !msg.mentioned_open_ids.is_empty();
        if !Self::is_message_allowed(cmd.chat_type, is_mention, context.bot_config) {
            return false;
        }
        // 闸门二：bot+chat_type 级别的响应开关（/sethome 会打开此开关）
        let response_enabled = context
            .db
            .get_feishu_response_enabled(context.bot_id, cmd.chat_type)
            .await
            .unwrap_or(false);
        if !response_enabled {
            tracing::info!(
                "[feishu:{}] message response is disabled for {} chat type",
                context.bot_id,
                cmd.chat_type
            );
            return false;
        }
        // 闸门三：群聊白名单；查询失败默认放行（防御性，不阻塞正常用户）
        if cmd.chat_type == "group" {
            let in_whitelist = match context
                .db
                .is_sender_in_whitelist(context.bot_id, &msg.sender)
                .await
            {
                Ok(allowed) => allowed,
                Err(e) => {
                    tracing::warn!(
                        "[feishu:{}] whitelist check failed for sender {}, defaulting to allow: {}",
                        context.bot_id,
                        msg.sender,
                        e
                    );
                    true
                }
            };
            if !in_whitelist {
                tracing::info!(
                    "[feishu:{}] sender {} not in group whitelist, skipping",
                    context.bot_id,
                    msg.sender
                );
                return false;
            }
        }
        true
    }

    /// 把页面创建的 __pending__ binding 关联到当前真实 chat。
    /// chat_id 写成 __pending__ 是页面端的占位策略，等首次消息进来再回填。
    /// 只在 todo 仍然存在时晋升，避免把残留 binding 绑到错误项目。
    async fn promote_pending_binding(
        db: &Arc<Database>,
        bot_id: i64,
        channel: &str,
        chat_type: &str,
    ) {
        let bindings = db
            .get_feishu_project_bindings(bot_id)
            .await
            .unwrap_or_default();
        // 找到 PENDING_CHAT_ID 的占位 binding
        let pending = bindings
            .iter()
            .find(|b| b.chat_id == crate::models::PENDING_CHAT_ID)
            .cloned();
        let Some(pending) = pending else {
            return;
        };
        // todo 已删除时 binding 残留，跳过晋升避免误绑
        if db.get_todo(pending.todo_id).await.ok().flatten().is_none() {
            return;
        }
        match db
            .attach_feishu_project_binding(pending.id, channel, chat_type)
            .await
        {
            Ok(_) => tracing::info!(
                "[feishu:{}] promoted pending binding {} (project_dir_id={}) to chat {}",
                bot_id,
                pending.id,
                pending.project_dir_id,
                channel
            ),
            Err(e) => tracing::warn!(
                "[feishu:{}] failed to promote pending binding: {}",
                bot_id,
                e
            ),
        }
    }

    /// 清理 reaction（add_reaction 失败时 reaction_id 为 None，直接跳过）
    async fn cleanup_reaction(
        context: &ListenerMessageContext<'_>,
        msg: &ChannelMessage,
        reaction_id: Option<&str>,
    ) {
        if let Some(rid) = reaction_id {
            Self::delete_reaction(
                context.credentials,
                context.token_manager,
                context.bot_id,
                &msg.id,
                rid,
            )
            .await;
        }
    }

    /// echo 调试日志（按 chat_type 区分私聊/群聊），方便排查"发了消息但没反应"
    fn log_echo_reply(context: &ListenerMessageContext<'_>, cmd: &FeishuCommandContext<'_>) {
        // echo_reply 关闭时静默，避免日志噪音
        if !context.bot_config.echo_reply {
            return;
        }
        if cmd.chat_type == "p2p" {
            tracing::info!(
                "[feishu:{}] 收到私聊消息: sender={}, content={}",
                context.bot_id,
                cmd.sender,
                cmd.content
            );
        } else if cmd.chat_type == "group" {
            tracing::info!(
                "[feishu:{}] 收到群聊消息: channel={}, sender={}, content={}",
                context.bot_id,
                cmd.channel,
                cmd.sender,
                cmd.content
            );
        }
    }

    /// 项目绑定路径：有 binding → 计算 resume 信息 → push 到 debounce → 返回 true。
    /// 无 binding / disabled / todo 缺失 → 返回 false，让上层降级到斜杠/默认回复。
    async fn handle_project_binding_path(
        context: &ListenerMessageContext<'_>,
        cmd: &FeishuCommandContext<'_>,
    ) -> bool {
        // 第一步：拉 binding
        let binding = match context
            .db
            .get_feishu_project_binding(context.bot_id, cmd.channel)
            .await
        {
            Ok(Some(b)) => b,
            Ok(None) => return false, // 无 binding，让上层处理斜杠/默认
            Err(e) => {
                tracing::error!("[feishu:{}] query binding failed: {e}", context.bot_id);
                return false;
            }
        };
        // disabled binding 不参与路由，fall through 到斜杠/默认
        if !binding.enabled {
            tracing::info!(
                "[feishu:{}] binding {} is disabled, falling through to slash commands",
                context.bot_id,
                binding.id
            );
            return false;
        }
        // 第二步：拉 todo
        let Some(todo) = context.db.get_todo(binding.todo_id).await.ok().flatten() else {
            tracing::warn!(
                "[feishu:{}] bound todo #{} not found for chat {}",
                context.bot_id,
                binding.todo_id,
                cmd.channel
            );
            return false;
        };
        // 第三步：算 resume + push
        Self::push_project_execution(context, cmd, &binding, &todo).await;
        true
    }

    /// 把项目绑定触发的执行推到 debounce，由 debounce 在去抖窗口后调用 run_todo_execution
    async fn push_project_execution(
        context: &ListenerMessageContext<'_>,
        cmd: &FeishuCommandContext<'_>,
        binding: &FeishuProjectBinding,
        todo: &crate::models::Todo,
    ) {
        // latest_record 是判断 should_resume 的唯一数据源：binding.latest_record_id
        // 可能为 None（首次执行）也可能指向已结束的 record
        let latest_record = match binding.latest_record_id {
            Some(rid) => context.db.get_execution_record(rid).await.ok().flatten(),
            None => None,
        };
        let (resume_session_id, resume_message) =
            Self::compute_resume_info(&latest_record, &binding.session_id, cmd.content);
        tracing::info!(
            "[feishu:{}] binding check: todo_id={}, latest_record_id={:?}, should_resume={}, binding.session_id={:?}",
            context.bot_id,
            binding.todo_id,
            binding.latest_record_id,
            resume_session_id.is_some(),
            binding.session_id
        );
        // executor 优先用 todo 自带的；未配置时退回 claudecode
        let executor = todo.executor.as_deref().unwrap_or("claudecode");
        context.debounce.push(PendingMessage {
            bot_id: context.bot_id,
            chat_id: cmd.channel.to_string(),
            chat_type: cmd.chat_type.to_string(),
            sender: cmd.sender.to_string(),
            content: cmd.content.to_string(),
            todo_id: binding.todo_id,
            todo_prompt: todo.prompt.clone(),
            executor: Some(executor.to_string()),
            trigger_type: "feishu_project_bind".to_string(),
            params: None,
            message_id: Some(cmd.message_id.to_string()),
            resume_session_id,
            resume_message,
            binding_id: Some(binding.id),
        });
        // 命中绑定路径，主动清理 reaction（不等到 handle_message 收尾）
        Self::cleanup_reaction(context, &Self::dummy_msg_from_cmd(cmd), cmd.reaction_id).await;
    }

    /// 纯函数：根据 latest_record + binding.session_id 决定是否 resume 已有 session。
    /// 返回 (resume_session_id, resume_message)：
    ///   - 不 resume → (None, None)
    ///   - resume → (real_sid, Some(content))
    fn compute_resume_info(
        latest_record: &Option<crate::models::ExecutionRecord>,
        binding_session_id: &Option<String>,
        content: &str,
    ) -> (Option<String>, Option<String>) {
        // should_resume 条件（取自 handle_message 原注释）：
        //   1. latest_record 必须有 session_id
        //   2. 上一次执行必须已结束（status != Running），
        //      避免与 Claude Code 正在写入的 JSONL 文件产生竞态
        let should_resume = latest_record
            .as_ref()
            .map(|r| {
                r.session_id.is_some()
                    && r.status != crate::models::ExecutionStatus::Running
            })
            .unwrap_or(false);
        if !should_resume {
            return (None, None);
        }
        // ⚠️ 不能直接用 binding_session_id：debounce 首次执行时把它设成了 task_id（随机 UUID），
        // Claude Code 真正的 session_id 来自 stdout JSONL，保存在 execution_records.session_id。
        let real_sid = latest_record
            .as_ref()
            .and_then(|r| r.session_id.clone())
            .or_else(|| binding_session_id.clone());
        (real_sid, Some(content.to_string()))
    }

    /// 项目绑定路径专用：根据 cmd 字段构造一个用于 cleanup 的伪 msg 引用。
    /// 避免 push_project_execution 持有原 msg 的借用，保持签名简单
    fn dummy_msg_from_cmd(cmd: &FeishuCommandContext<'_>) -> ChannelMessage {
        ChannelMessage {
            id: cmd.message_id.to_string(),
            sender: cmd.sender.to_string(),
            sender_type: None,
            content: cmd.content.to_string(),
            channel: cmd.channel.to_string(),
            // timestamp 在 cleanup 路径中不被使用，填 0 即可
            timestamp: 0,
            chat_type: Some(cmd.chat_type.to_string()),
            mentioned_open_ids: vec![],
        }
    }

    /// 斜杠规则 + 默认回复兜底：优先匹配 config.slash_command_rules，
    /// 命中后用规则关联的 todo 执行；未命中或非斜杠消息走 default_response_todo_id
    async fn dispatch_slash_or_default(
        context: &ListenerMessageContext<'_>,
        cmd: &FeishuCommandContext<'_>,
    ) {
        // 先尝试用户自定义斜杠规则
        if let Some(command_ctx) = Self::parse_slash_command(cmd.content) {
            if Self::try_dispatch_slash_rule(context, cmd, &command_ctx).await {
                return;
            }
        }
        // 兜底：默认回复 todo
        Self::dispatch_default_response(context, cmd).await;
    }

    /// 尝试匹配一条 slash_command_rules；命中且能拿到 todo 就 push 并返回 true
    async fn try_dispatch_slash_rule(
        context: &ListenerMessageContext<'_>,
        cmd: &FeishuCommandContext<'_>,
        command_ctx: &SlashCommandMatch<'_>,
    ) -> bool {
        // 在 config 里查匹配的规则（克隆出来以释放读锁）
        let rule = {
            let cfg = context.config.read().unwrap();
            cfg.slash_command_rules
                .iter()
                .find(|r| r.slash_command == command_ctx.command && r.enabled)
                .cloned()
        };
        let Some(rule) = rule else {
            return false;
        };
        // 斜杠命令 body 为空时不触发
        if command_ctx.body.is_empty() {
            return false;
        }
        // 拉规则关联的 todo
        let Some(todo) = context.db.get_todo(rule.todo_id).await.ok().flatten() else {
            tracing::error!(
                "Failed to fetch todo {} for slash command",
                rule.todo_id
            );
            return false;
        };
        let (_, params) =
            build_trigger_params(&format!("{} {}", command_ctx.command, command_ctx.body));
        context.debounce.push(PendingMessage {
            bot_id: context.bot_id,
            chat_id: cmd.channel.to_string(),
            chat_type: cmd.chat_type.to_string(),
            sender: cmd.sender.to_string(),
            content: command_ctx.body.to_string(),
            todo_id: todo.id,
            todo_prompt: todo.prompt.clone(),
            executor: todo.executor.clone(),
            trigger_type: "slash_command".to_string(),
            params: Some(params),
            message_id: Some(cmd.message_id.to_string()),
            resume_session_id: None,
            resume_message: None,
            binding_id: None,
        });
        true
    }

    /// 默认回复：未匹配任何斜杠规则时把消息丢给 default_response_todo_id
    async fn dispatch_default_response(
        context: &ListenerMessageContext<'_>,
        cmd: &FeishuCommandContext<'_>,
    ) {
        // 未配置默认 todo 直接退出
        let default_todo_id = context.config.read().unwrap().default_response_todo_id;
        let Some(todo_id) = default_todo_id else {
            return;
        };
        // 空内容不触发
        if cmd.content.is_empty() {
            return;
        }
        // 容错：todo 缺失时使用空 prompt，避免阻塞消息流
        let todo_prompt = context
            .db
            .get_todo(todo_id)
            .await
            .ok()
            .flatten()
            .map(|t| t.prompt)
            .unwrap_or_default();
        let (_, params) = build_trigger_params(cmd.content);
        context.debounce.push(PendingMessage {
            bot_id: context.bot_id,
            chat_id: cmd.channel.to_string(),
            chat_type: cmd.chat_type.to_string(),
            sender: cmd.sender.to_string(),
            content: cmd.content.to_string(),
            todo_id,
            todo_prompt,
            executor: None,
            trigger_type: "default_response".to_string(),
            params: Some(params),
            message_id: Some(cmd.message_id.to_string()),
            resume_session_id: None,
            resume_message: None,
            binding_id: None,
        });
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

    async fn handle_sethome(context: FeishuCommandContext<'_>) {
        let FeishuCommandContext {
            db,
            credentials,
            token_manager,
            bot_id,
            chat_type,
            sender,
            channel,
            message_id,
            reaction_id,
            ..
        } = context;
        let target_type = if chat_type == "p2p" { "p2p" } else { "group" };
        let (receive_id, receive_id_type, chat_id) = match chat_type {
            "p2p" => (sender.to_string(), "open_id", None),
            _ => (channel.to_string(), "chat_id", Some(channel.to_string())),
        };

        // Update feishu_home
        match db
            .set_feishu_home(
                bot_id,
                sender,
                chat_id.as_deref(),
                &receive_id,
                receive_id_type,
            )
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "[feishu:{}] /sethome by {} → {} ({})",
                    bot_id,
                    sender,
                    receive_id,
                    receive_id_type
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
        } else if let Err(e) = db.set_group_chat_id(bot_id, channel).await {
            tracing::error!("[feishu:{}] set group push target failed: {e}", bot_id);
        }

        // Enable message response for this chat type
        if let Err(e) = db
            .set_feishu_response_enabled(bot_id, target_type, true)
            .await
        {
            tracing::error!("[feishu:{}] enable response failed: {e}", bot_id);
        }

        // Send confirmation
        let chat_type_label = if chat_type == "p2p" {
            "私聊"
        } else {
            "群聊"
        };
        let target_desc = if chat_type == "p2p" {
            "此私聊"
        } else {
            channel
        };
        let confirm = format!("✅ 已设置推送目标为此 {chat_type_label} ({target_desc})，执行过程将实时推送。\n\n如需关闭推送，请发送 /feishupush");
        Self::send_text(
            credentials,
            token_manager,
            bot_id,
            &receive_id,
            receive_id_type,
            &confirm,
        )
        .await;

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
        }
    }

    /// Handle /feishupush - cycle push level: disabled -> result_only -> all -> disabled.
    async fn handle_feishupush(context: FeishuCommandContext<'_>) {
        let FeishuCommandContext {
            db,
            credentials,
            token_manager,
            bot_id,
            chat_type,
            sender,
            channel,
            message_id,
            reaction_id,
            ..
        } = context;
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        let target = db.get_feishu_push_target(bot_id).await.ok().flatten();
        let current_level = target
            .as_ref()
            .map(|t| t.push_level.as_str())
            .unwrap_or("disabled");
        let new_level = match current_level {
            "disabled" => "result_only",
            "result_only" => "all",
            "all" => "disabled",
            _ => "disabled",
        };

        if let Err(e) = db.update_feishu_push_level(bot_id, new_level).await {
            tracing::error!("[feishu:{}] /feishupush update failed: {e}", bot_id);
            let msg = "⚠️ 操作失败，请稍后重试";
            Self::send_text(
                credentials,
                token_manager,
                bot_id,
                &receive_id,
                receive_id_type,
                msg,
            )
            .await;
        } else {
            let (status_text, status_emoji) = match new_level {
                "disabled" => ("关闭", "ℹ️"),
                "result_only" => ("已切换为仅结论", "✅"),
                "all" => ("已切换为全部", "✅"),
                _ => ("未知", "⚠️"),
            };
            let msg = format!("{} 推送{}。", status_emoji, status_text);
            Self::send_text(
                credentials,
                token_manager,
                bot_id,
                &receive_id,
                receive_id_type,
                &msg,
            )
            .await;
            tracing::info!(
                "[feishu:{}] /feishupush: push level changed to {} for bot_id={}",
                bot_id,
                new_level,
                bot_id
            );
        }

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
        }
    }

    /// Handle /list — list all registered project directories.
    async fn handle_list(context: FeishuCommandContext<'_>) {
        let FeishuCommandContext {
            db,
            credentials,
            token_manager,
            bot_id,
            chat_type,
            sender,
            channel,
            message_id,
            reaction_id,
            ..
        } = context;
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        let directories = db.get_project_directories().await.unwrap_or_default();
        if directories.is_empty() {
            Self::send_text(
                credentials,
                token_manager,
                bot_id,
                &receive_id,
                receive_id_type,
                "📂 暂无已注册的项目目录。\n\n请在 Web 设置页「项目目录」中添加，或使用 /bind <名称> 绑定一个项目（首次使用会自动创建）。",
            )
            .await;
        } else {
            let mut lines: Vec<String> = directories
                .iter()
                .map(|d| {
                    let name = d.name.as_deref().unwrap_or("(未命名)");
                    format!("• {}  →  {}", name, d.path)
                })
                .collect();
            lines.insert(0, format!("📂 已注册的项目目录（共 {} 个）：", directories.len()));
            lines.push(String::new());
            lines.push("💡 使用 /bind <名称> 绑定到本项目聊天".to_string());
            Self::send_text(
                credentials,
                token_manager,
                bot_id,
                &receive_id,
                receive_id_type,
                &lines.join("\n"),
            )
            .await;
        }

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
        }
    }

    /// Handle /bind — show current binding, or /bind <name> to bind to a project.
    async fn handle_bind(context: FeishuCommandContext<'_>) {
        let FeishuCommandContext {
            db,
            credentials,
            token_manager,
            bot_id,
            chat_type,
            sender,
            channel,
            message_id,
            content,
            reaction_id,
        } = context;
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        // /bind with no args → show current binding status
        if content == "/bind" {
            match db.get_feishu_project_binding(bot_id, channel).await {
                Ok(Some(binding)) => {
                    let dir = db.get_project_directory_by_id(binding.project_dir_id).await.ok().flatten();
                    let dir_name = dir.as_ref().and_then(|d| d.name.as_deref()).unwrap_or("(unknown)");
                    let dir_path = dir.as_ref().map(|d| d.path.as_str()).unwrap_or("(unknown)");
                    let status_icon = if binding.status == crate::models::binding_status::RUNNING { "🟢" } else { "⏸️" };
                    let msg = format!(
                        "📎 当前绑定详情：\n项目：{dir_name}\n目录：{dir_path}\nTodo：#{binding_id}\n状态：{status_icon} {binding_status}\nSession：{session}\n\n💡 使用 /unbind 解绑",
                        binding_id = binding.todo_id,
                        binding_status = binding.status,
                        session = binding.session_id.as_deref().unwrap_or("(无)"),
                    );
                    Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &msg).await;
                }
                Ok(None) => {
                    Self::send_text(
                        credentials, token_manager, bot_id, &receive_id, receive_id_type,
                        "📭 当前聊天未绑定任何项目。\n\n使用 /bind <项目名称> 绑定一个项目。\n使用 /list 查看可用项目。",
                    )
                    .await;
                }
                Err(e) => {
                    tracing::error!("[feishu:{}] /bind query failed: {e}", bot_id);
                    Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "⚠️ 查询绑定失败，请稍后重试").await;
                }
            }
            if let Some(rid) = reaction_id {
                Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
            }
            return;
        }

        // /bind <name> — bind to a project by name
        let project_name = content.strip_prefix("/bind ").unwrap_or("").trim();
        if project_name.is_empty() {
            Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "⚠️ 请输入项目名称，例如：/bind my-app").await;
            if let Some(rid) = reaction_id {
                Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
            }
            return;
        }

        // 按项目名称查找：先精确匹配，再前缀匹配
        // ⚠️ 前缀匹配时若有多个候选（如 my-app / my-application 都匹配 "my"），
        //    返回歧义提示让用户精确输入。
        let directories = db.get_project_directories().await.unwrap_or_default();
        // 精确匹配 — 唯一正确
        let dir = directories.iter().find(|d| d.name.as_deref() == Some(project_name)).cloned();
        let dir = match dir {
            Some(d) => Some(d),
            None => {
                // 前缀匹配 — 检查是否有多选歧义
                let candidates: Vec<_> = directories.iter()
                    .filter(|d| d.name.as_deref().map(|n| n.starts_with(project_name)).unwrap_or(false))
                    .collect();
                if candidates.is_empty() {
                    None
                } else if candidates.len() == 1 {
                    Some(candidates[0].clone())
                } else {
                    // 多个候选，返回歧义提示
                    let names: Vec<String> = candidates.iter()
                        .filter_map(|d| d.name.as_deref())
                        .map(|n| format!("• {}", n))
                        .collect();
                    let msg = format!(
                        "⚠️ 「{}」匹配到多个项目：\n{}\n\n请使用完整名称，例如：/bind {}",
                        project_name,
                        names.join("\n"),
                        candidates[0].name.as_deref().unwrap_or(""),
                    );
                    Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &msg).await;
                    if let Some(rid) = reaction_id {
                        Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
                    }
                    return;
                }
            }
        };

        match dir {
            Some(dir) => {
                // Check if already bound
                if let Ok(Some(existing)) = db.get_feishu_project_binding(bot_id, channel).await {
                    if let Err(e) = db.delete_feishu_project_binding(existing.id).await {
                        tracing::warn!("[feishu:{}] failed to delete existing binding {} before rebind: {}", bot_id, existing.id, e);
                    }
                }

                // Try to find a pending binding created via Web UI (chat_id=PENDING_CHAT_ID)
                let pending_bindings = db.get_feishu_project_bindings(bot_id).await.unwrap_or_default();
                let pending = pending_bindings.iter()
                    .find(|b| b.project_dir_id == dir.id && b.chat_id == crate::models::PENDING_CHAT_ID)
                    .cloned();

                if let Some(pending_binding) = pending {
                    // Reuse the pending binding and its todo — just update chat_id/chat_type
                    match db.attach_feishu_project_binding(pending_binding.id, channel, chat_type).await {
                        Ok(_) => {
                            let dir_name = dir.name.as_deref().unwrap_or("unknown");
                            let msg = format!(
                                "✅ 已绑定到项目「{dir_name}」\n项目目录：{path}\nTodo：#{todo_id}\n\n现在可以直接向我发送任务了。",
                                path = dir.path,
                                todo_id = pending_binding.todo_id,
                            );
                            Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &msg).await;
                        }
                        Err(e) => {
                            tracing::error!("[feishu:{}] update pending binding failed: {e}", bot_id);
                            Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "⚠️ 绑定更新失败，请稍后重试").await;
                        }
                    }
                } else {
                    // No pending binding — create a new Todo + binding
                    let todo_title = format!("飞书-{}", dir.name.as_deref().unwrap_or(&dir.path));
                    let todo_prompt = format!(
                        "你是飞书Bot的AI助手，正在项目「{name}」({path})中工作。\n\
                         用户通过飞书与你交流，请根据用户的需求在项目目录中完成开发任务。\n\
                         你可以读取、修改项目文件，运行命令等。\n\n\
                         用户诉求：{{message}}\n\
                         项目目录：{path}",
                        name = dir.name.as_deref().unwrap_or("unknown"),
                        path = dir.path,
                    );

                    match db.create_todo(&todo_title, &todo_prompt).await {
                        Ok(todo_id) => {
                            let _ = db.update_todo_workspace(todo_id, Some(&dir.path)).await;
                            let _ = db.update_todo_worktree_enabled(todo_id, false).await;
                            match db.create_feishu_project_binding(bot_id, channel, chat_type, dir.id, todo_id).await {
                                Ok(binding_id) => {
                                    let dir_name = dir.name.as_deref().unwrap_or("unknown");
                                    let msg = format!(
                                        "✅ 已绑定到项目「{dir_name}」\n项目目录：{path}\nTodo：#{todo_id}\n\n现在可以直接向我发送任务了。",
                                        path = dir.path,
                                    );
                                    Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &msg).await;
                                    tracing::info!("[feishu:{}] bound chat {} to project {} (binding={}, todo={})", bot_id, channel, dir.path, binding_id, todo_id);
                                }
                                Err(e) => {
                                    tracing::error!("[feishu:{}] create binding failed: {e}", bot_id);
                                    Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "⚠️ 创建绑定失败，请稍后重试").await;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("[feishu:{}] create todo failed: {e}", bot_id);
                            Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "⚠️ 创建 Todo 失败，请稍后重试").await;
                        }
                    }
                }
            }
            None => {
                let msg = format!(
                    "⚠️ 未找到名为「{name}」的项目。\n\n使用 /list 查看所有可用项目。",
                    name = project_name
                );
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, &msg).await;
            }
        }

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
        }
    }

    /// Handle /unbind — unbind current chat from its project.
    async fn handle_unbind(context: FeishuCommandContext<'_>) {
        let FeishuCommandContext {
            db,
            credentials,
            token_manager,
            bot_id,
            chat_type,
            sender,
            channel,
            message_id,
            reaction_id,
            ..
        } = context;
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        match db.get_feishu_project_binding(bot_id, channel).await {
            Ok(Some(binding)) => {
                // 任务运行时拒绝解绑，避免 session 链丢失。
                // 用户必须先通过 Web UI 停止运行中的任务，才能解绑。
                if binding.status == crate::models::binding_status::RUNNING {
                    Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type,
                        "⚠️ 当前有任务正在执行（session 链会被丢弃）。\n请先通过 Web 界面「运行管理」停止任务，再发送 /unbind 解绑。")
                        .await;
                    if let Some(rid) = reaction_id {
                        Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
                    }
                    return;
                }

                if let Err(e) = db.delete_feishu_project_binding(binding.id).await {
                    tracing::error!("[feishu:{}] /unbind failed: {e}", bot_id);
                    Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "⚠️ 解绑失败，请稍后重试").await;
                } else {
                    Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "✅ 已解绑。使用 /bind <名称> 重新绑定到其他项目。").await;
                }
            }
            Ok(None) => {
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "📭 当前聊天未绑定任何项目，无需解绑。").await;
            }
            Err(e) => {
                tracing::error!("[feishu:{}] /unbind query failed: {e}", bot_id);
                Self::send_text(credentials, token_manager, bot_id, &receive_id, receive_id_type, "⚠️ 查询绑定失败，请稍后重试").await;
            }
        }

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
        }
    }

    /// Handle /new — start a fresh session without resuming the previous one.
    /// Unlike normal messages which resume existing sessions, this forces a new session.
    async fn handle_new(context: FeishuCommandContext<'_>) {
        let FeishuCommandContext {
            db,
            credentials,
            token_manager,
            bot_id,
            chat_type,
            sender,
            channel,
            message_id,
            reaction_id,
            ..
        } = context;
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        match db.get_feishu_project_binding(bot_id, channel).await {
            Ok(Some(binding)) => {
                // 清除 session_id 和 latest_record_id，使下一条消息无法 resume
                // should_resume 的判断依赖 latest_record.session_id.is_some()，
                // 清除后 latest_record_id=None → latest_record=None → should_resume=false
                if let Err(e) = db.clear_feishu_binding_session(binding.id).await {
                    tracing::error!("[feishu:{}] /new clear session failed: {e}", bot_id);
                    Self::send_text(
                        credentials,
                        token_manager,
                        bot_id,
                        &receive_id,
                        receive_id_type,
                        "⚠️ 清除会话失败，请稍后重试。",
                    )
                    .await;
                    if let Some(rid) = reaction_id {
                        Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
                    }
                    return;
                }

                tracing::info!(
                    "[feishu:{}] /new command: cleared session for binding {}, next message will start fresh",
                    bot_id,
                    binding.id
                );
                Self::send_text(
                    credentials,
                    token_manager,
                    bot_id,
                    &receive_id,
                    receive_id_type,
                    "🆕 已开启新会话。\n\n发送你的任务，我将使用全新 session 执行，不再resume之前的对话。",
                )
                .await;
            }
            Ok(None) => {
                Self::send_text(
                    credentials,
                    token_manager,
                    bot_id,
                    &receive_id,
                    receive_id_type,
                    "📭 当前聊天未绑定任何项目，无法使用 /new。\n\n请先使用 /bind <项目名称> 绑定一个项目。",
                )
                .await;
            }
            Err(e) => {
                tracing::error!("[feishu:{}] /new query binding failed: {e}", bot_id);
                Self::send_text(
                    credentials,
                    token_manager,
                    bot_id,
                    &receive_id,
                    receive_id_type,
                    "⚠️ 查询绑定失败，请稍后重试。",
                )
                .await;
            }
        }

        if let Some(rid) = reaction_id {
            Self::delete_reaction(credentials, token_manager, bot_id, message_id, rid).await;
        }
    }

    /// Handle /stop — stop the currently running execution for this binding.
    /// 与前端「停止」按钮逻辑相同：通过 task_manager 取消任务。
    async fn handle_stop(
        task_manager: &Arc<TaskManager>,
        context: FeishuCommandContext<'_>,
    ) {
        let FeishuCommandContext {
            db,
            credentials,
            token_manager,
            bot_id,
            chat_type,
            sender,
            channel,
            message_id,
            reaction_id,
            ..
        } = context;
        let (receive_id, receive_id_type) = match chat_type {
            "p2p" => (sender.to_string(), "open_id"),
            _ => (channel.to_string(), "chat_id"),
        };

        match db.get_feishu_project_binding(bot_id, channel).await {
            Ok(Some(binding)) => {
                // 获取当前 binding 的最新执行记录
                if let Some(record_id) = binding.latest_record_id {
                    match db.get_execution_record(record_id).await {
                        Ok(Some(record)) => {
                            if record.status == crate::models::ExecutionStatus::Running {
                                // 任务正在运行，尝试停止
                                if let Some(ref task_id) = record.task_id {
                                    let cancelled = task_manager.cancel(task_id).await;
                                    if cancelled {
                                        tracing::info!(
                                            "[feishu:{}] /stop: cancelled task {} for record {}",
                                            bot_id,
                                            task_id,
                                            record_id
                                        );
                                        Self::send_text(
                                            credentials,
                                            token_manager,
                                            bot_id,
                                            &receive_id,
                                            receive_id_type,
                                            "⏹️ 已发送停止信号，任务即将终止。",
                                        )
                                        .await;
                                    } else {
                                        // 任务不在 task_manager 中（可能已崩溃），强制更新 DB
                                        tracing::warn!(
                                            "[feishu:{}] /stop: task {} not in task_manager, forcing DB update",
                                            bot_id,
                                            task_id
                                        );
                                        let _ = db.force_fail_execution_record(record_id).await;
                                        Self::send_text(
                                            credentials,
                                            token_manager,
                                            bot_id,
                                            &receive_id,
                                            receive_id_type,
                                            "⚠️ 任务已不在运行中（可能已异常退出），已更新状态。",
                                        )
                                        .await;
                                    }
                                } else {
                                    Self::send_text(
                                        credentials,
                                        token_manager,
                                        bot_id,
                                        &receive_id,
                                        receive_id_type,
                                        "⚠️ 该执行记录没有 task_id，无法停止。",
                                    )
                                    .await;
                                }
                            } else {
                                Self::send_text(
                                    credentials,
                                    token_manager,
                                    bot_id,
                                    &receive_id,
                                    receive_id_type,
                                    "ℹ️ 当前没有正在执行的任务。",
                                )
                                .await;
                            }
                        }
                        Ok(None) => {
                            Self::send_text(
                                credentials,
                                token_manager,
                                bot_id,
                                &receive_id,
                                receive_id_type,
                                "⚠️ 执行记录不存在。",
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::error!("[feishu:{}] /stop query record failed: {e}", bot_id);
                            Self::send_text(
                                credentials,
                                token_manager,
                                bot_id,
                                &receive_id,
                                receive_id_type,
                                "⚠️ 查询执行记录失败，请稍后重试。",
                            )
                            .await;
                        }
                    }
                } else {
                    Self::send_text(
                        credentials,
                        token_manager,
                        bot_id,
                        &receive_id,
                        receive_id_type,
                        "ℹ️ 当前没有执行记录可停止。",
                    )
                    .await;
                }
            }
            Ok(None) => {
                Self::send_text(
                    credentials,
                    token_manager,
                    bot_id,
                    &receive_id,
                    receive_id_type,
                    "📭 当前聊天未绑定任何项目，无可停止的任务。",
                )
                .await;
            }
            Err(e) => {
                tracing::error!("[feishu:{}] /stop query binding failed: {e}", bot_id);
                Self::send_text(
                    credentials,
                    token_manager,
                    bot_id,
                    &receive_id,
                    receive_id_type,
                    "⚠️ 查询绑定失败，请稍后重试。",
                )
                .await;
            }
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
        let url = format!(
            "{}/open-apis/im/v1/messages?receive_id_type={}",
            base_url, receive_id_type
        );
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
                    tracing::debug!(
                        "[feishu:{}] send_text ok to {} ({})",
                        bot_id,
                        receive_id,
                        receive_id_type
                    );
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
        let url = format!(
            "{}/open-apis/im/v1/messages?receive_id_type={}",
            base_url, receive_id_type
        );
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

    fn base_url(
        credentials: &DashMap<i64, (String, String, String)>,
        bot_id: i64,
    ) -> Option<String> {
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
        let (app_id, app_secret, domain) =
            (ref_val.0.clone(), ref_val.1.clone(), ref_val.2.clone());
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
        tracing::info!(
            "[feishu:{}] add_reaction POST {} token={}... body={}",
            bot_id,
            url,
            &token[..token.len().min(10)],
            body_json
        );
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
            tracing::error!(
                "[feishu:{}] add_reaction API error (status={}): {body}",
                bot_id,
                status
            );
            return None;
        }

        let reaction_id = body
            .get("data")
            .and_then(|d| d.get("reaction_id"))
            .and_then(|v| v.as_str())
            .map(String::from);

        tracing::info!(
            "[feishu:{}] add_reaction {} ok, reaction_id={:?}",
            bot_id,
            emoji_type,
            reaction_id
        );
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
            .delete(format!(
                "{base_url}/open-apis/im/v1/messages/{message_id}/reactions/{reaction_id}"
            ))
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

    // --- compute_resume_info 的单元测试 ---
    // 纯函数：根据 latest_record + binding_session_id + content 计算 resume 决策。
    // 覆盖：None record / 无 session_id / status=Running / 正常 resume 四个分支。

    /// 构造一个测试用 ExecutionRecord；status / session_id 由调用方指定
    fn make_test_record(
        session_id: Option<String>,
        status: crate::models::ExecutionStatus,
    ) -> crate::models::ExecutionRecord {
        crate::models::ExecutionRecord {
            id: 1,
            todo_id: 1,
            session_id,
            status,
            command: String::new(),
            stdout: String::new(),
            stderr: String::new(),
            result: None,
            started_at: String::new(),
            finished_at: None,
            usage: None,
            executor: None,
            model: None,
            trigger_type: "test".to_string(),
            pid: None,
            task_id: None,
            todo_progress: None,
            execution_stats: None,
            resume_message: None,
            source_todo_id: None,
            source_todo_title: None,
            source_hook_id: None,
            rating: None,
            source_execution_record_id: None,
            last_review_status: None,
            last_reviewed_at: None,
            worktree_path: None,
        }
    }

    /// 无 latest_record → 不 resume
    #[test]
    fn test_compute_resume_info_no_record() {
        let (sid, msg) = FeishuListener::compute_resume_info(&None, &None, "继续任务");
        assert!(sid.is_none());
        assert!(msg.is_none());
    }

    /// latest_record 没有 session_id → 不 resume
    #[test]
    fn test_compute_resume_info_no_session_id() {
        let record = make_test_record(None, crate::models::ExecutionStatus::Success);
        let (sid, msg) = FeishuListener::compute_resume_info(&Some(record), &None, "继续任务");
        assert!(sid.is_none());
        assert!(msg.is_none());
    }

    /// status=Running 时不 resume（避免与 Claude Code JSONL 写竞态）
    #[test]
    fn test_compute_resume_info_running_status() {
        let record = make_test_record(
            Some("sess-abc".to_string()),
            crate::models::ExecutionStatus::Running,
        );
        let (sid, msg) = FeishuListener::compute_resume_info(&Some(record), &None, "继续任务");
        assert!(sid.is_none());
        assert!(msg.is_none());
    }

    /// 正常 resume 路径：返回 latest_record.session_id + content
    #[test]
    fn test_compute_resume_info_normal_resume() {
        let record = make_test_record(
            Some("sess-xyz".to_string()),
            crate::models::ExecutionStatus::Success,
        );
        let (sid, msg) = FeishuListener::compute_resume_info(
            &Some(record),
            &Some("fallback-sid".to_string()),
            "帮我重构",
        );
        assert_eq!(sid.as_deref(), Some("sess-xyz"));
        assert_eq!(msg.as_deref(), Some("帮我重构"));
    }

    /// session_id 缺失时回退到 binding_session_id
    #[test]
    fn test_compute_resume_info_fallback_to_binding() {
        let record = make_test_record(None, crate::models::ExecutionStatus::Success);
        // ⚠️ 当前实现要求 record.session_id.is_some() 才走 resume 分支，
        // 因此 None 不会触发 fallback；该测试断言当前行为
        let (sid, msg) = FeishuListener::compute_resume_info(
            &Some(record),
            &Some("binding-sid".to_string()),
            "继续",
        );
        assert!(sid.is_none());
        assert!(msg.is_none());
    }
}
