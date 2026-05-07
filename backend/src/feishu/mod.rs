pub mod channel;
pub mod codec;
pub mod config;
pub mod message;
pub mod sdk;

pub use channel::FeishuChannelService;
pub use config::{FeishuConfig, FeishuConnectionMode, FeishuDomain};
pub use message::ChannelMessage;

/// Create a new Feishu channel service from config.
pub fn create_channel(config: FeishuConfig) -> FeishuChannelService {
    FeishuChannelService::new(config)
}
