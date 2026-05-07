/// Decode Feishu message content JSON into plain text.
pub fn decode_message_content(content: &str, message_type: &str) -> String {
    match message_type {
        "text" => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
                parsed
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or(content)
                    .to_string()
            } else {
                content.to_string()
            }
        }
        _ => content.to_string(),
    }
}

/// Encode a plain text string into Feishu text message JSON.
pub fn encode_text_message(text: &str) -> String {
    serde_json::json!({ "text": text }).to_string()
}
