//! FeishuPlatform 集成测试。
//!
//! M3 v1 验证目标：
//! - FeishuPlatform 实现 Channel trait 的语义正确（reply/send 参数解析）
//! - FeishuPlatform 实现 TypingIndicator trait（带/不带 message_id 都能调用）
//! - FeishuPlatform cast 成 Arc<dyn Channel> + Arc<dyn TypingIndicator> 都成功
//! - TypingGuard.stop() 在 HTTP 失败时也不 panic
//!
//! 不验证（v1 暂不支持 base URL override）：
//! - 实际 HTTP 请求格式（base URL 写死 https://open.feishu.cn）
//! - tenant_token 缓存复用
//!
//! 真实 HTTP 集成测试推迟到 v2（届时 FeishuPlatform 支持 base_url
//! 注入，可以用 wiremock 真正拦截 HTTP 调用）。

use std::sync::Arc;

use ntd_connect::channel::{Channel, MessageHandler};
use ntd_connect::error::Error;
use ntd_connect::http::SharedHttpClient;
use ntd_connect::platform::feishu::{FeishuConfig, FeishuPlatform, FeishuDomain};
use ntd_connect::types::{FeishuChatType, OutgoingContent, ReplyContext, ReplyTarget};
use ntd_connect::typing::TypingIndicator;

/// 构造一个测试用 FeishuPlatform。
fn make_platform() -> FeishuPlatform {
    FeishuPlatform::new(
        FeishuConfig {
            app_id: "test_app_id".into(),
            app_secret: "test_app_secret".into(),
            domain: FeishuDomain::Feishu,
            bot_open_id: Some("ou_bot_self".into()),
        },
        SharedHttpClient::new(),
    )
}

/// Channel::reply 在 P2P 场景的 URL 路径分支验证：
/// 请求会失败（base URL 不可达），但返回的应是 Platform 类错误
/// 而非 panic，证明走到了 extract_feishu_target + send_message 分支。
#[tokio::test(flavor = "current_thread")]
async fn test_reply_p2p_branch() {
    let platform = make_platform();
    let target = ReplyTarget::feishu("ou_user_a", None, FeishuChatType::P2p);
    let res = platform
        .reply(
            &ReplyContext::default(),
            target,
            OutgoingContent::Text("hi".into()),
        )
        .await;
    let err = res.unwrap_err();
    assert!(
        matches!(err, Error::Platform(_)),
        "expected Platform error, got {err:?}"
    );
}

/// Channel::reply 群聊场景：extract_feishu_target 派生 chat_id 类型。
#[tokio::test(flavor = "current_thread")]
async fn test_reply_group_branch() {
    let platform = make_platform();
    let target = ReplyTarget::feishu("oc_chat_xyz", None, FeishuChatType::Group);
    let res = platform
        .reply(
            &ReplyContext::default(),
            target,
            OutgoingContent::Text("hi".into()),
        )
        .await;
    assert!(matches!(res.unwrap_err(), Error::Platform(_)));
}

/// TypingIndicator::start_typing 没 message_id 时返回 noop guard。
/// 主动推送场景（send 到 chat 但不 reply 具体消息）属于这种情况，
/// TypingIndicator 没必要打 reaction。
#[tokio::test(flavor = "current_thread")]
async fn test_start_typing_no_message_id_returns_noop() {
    let platform = make_platform();
    let target = ReplyTarget::feishu("ou_user_a", None, FeishuChatType::P2p);
    let guard = platform
        .start_typing(&ReplyContext::default(), &target)
        .await
        .unwrap();
    guard.stop().await;
}

/// TypingIndicator::start_typing 带 message_id：内部尝试 HTTP 调用，
/// 失败不 panic，返回 Ok(TypingGuard::noop())（typing 启动失败不阻塞主流程）。
#[tokio::test(flavor = "current_thread")]
async fn test_start_typing_with_message_id_swallows_error() {
    let platform = make_platform();
    let target = ReplyTarget::feishu(
        "oc_chat_xyz",
        Some("om_msg_001".into()),
        FeishuChatType::Group,
    );
    let guard = platform
        .start_typing(&ReplyContext::default(), &target)
        .await
        .unwrap();
    // stop() 必须立即返回（noop），不 await 真实 HTTP。
    guard.stop().await;
}

/// FeishuPlatform 必须能 cast 成 dyn Channel 和 dyn TypingIndicator。
/// 这是 dispatcher 探测能力的入口。
#[test]
fn test_platform_trait_objects() {
    let platform = Arc::new(make_platform());
    let as_channel: Arc<dyn Channel> = platform.clone();
    assert_eq!(as_channel.name(), "feishu");

    // Channel::as_typing_indicator 必须返回 Some。
    let ti: Option<&dyn TypingIndicator> = as_channel.as_typing_indicator();
    assert!(ti.is_some(), "FeishuPlatform 必须报告 typing 能力");
}

/// Channel::start 注册 handler 不 panic（M3 v1 stub 实现）。
///
/// M3 v1 的 start() 只 log 一行 + 返回 Ok，真正的 WS 连接待 v2 接
/// backend `feishu/sdk/ws_client.rs`。
#[tokio::test(flavor = "current_thread")]
async fn test_channel_start_returns_ok() {
    let platform = Arc::new(make_platform());
    let dyn_ch: Arc<dyn Channel> = platform.clone();

    struct StubHandler;
    #[async_trait::async_trait]
    impl MessageHandler for StubHandler {
        async fn on_message(
            &self,
            _ch: Arc<dyn Channel>,
            _msg: ntd_connect::types::IncomingMessage,
        ) -> ntd_connect::error::Result<()> {
            Ok(())
        }
    }

    dyn_ch
        .start(Arc::new(StubHandler))
        .await
        .expect("Channel::start v1 stub must return Ok");
}

/// Channel::stop 不 panic（v1 stub 实现，cancel handle 即可）。
#[tokio::test(flavor = "current_thread")]
async fn test_channel_stop_returns_ok() {
    let platform = Arc::new(make_platform());
    let dyn_ch: Arc<dyn Channel> = platform.clone();
    dyn_ch.stop().await.expect("Channel::stop must return Ok");
}
