/// A decoded Feishu message received from the WebSocket event stream.
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,
    pub sender_type: Option<String>,
    pub content: String,
    pub channel: String,
    pub timestamp: u64,
    pub chat_type: Option<String>,
    pub mentioned_open_ids: Vec<String>,
}
