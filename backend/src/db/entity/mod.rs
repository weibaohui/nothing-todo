pub mod agent_bots;
pub mod execution_records;
pub mod feishu_homes;
pub mod feishu_history_chats;
pub mod feishu_messages;
pub mod feishu_push_targets;
pub mod feishu_response_config;
pub mod tags;
pub mod todo_tags;
pub mod todos;

pub mod prelude {
    pub use super::agent_bots::Entity as AgentBots;
    pub use super::execution_records::Entity as ExecutionRecords;
    pub use super::feishu_homes::Entity as FeishuHomes;
    pub use super::feishu_history_chats::Entity as FeishuHistoryChats;
    pub use super::feishu_messages::Entity as FeishuMessages;
    pub use super::feishu_push_targets::Entity as FeishuPushTargets;
    pub use super::feishu_response_config::Entity as FeishuResponseConfig;
    pub use super::tags::Entity as Tags;
    pub use super::todo_tags::Entity as TodoTags;
    pub use super::todos::Entity as Todos;
}
