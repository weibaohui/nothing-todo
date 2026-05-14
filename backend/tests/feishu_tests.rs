//! Tests for Feishu/Lark module - codec, message types, and SDK error handling

#[cfg(test)]
mod codec_tests {
    use ntd::feishu::codec::{decode_message_content, encode_text_message};

    #[test]
    fn test_decode_text_message_content() {
        let content = r#"{"text":"Hello world"}"#;
        let result = decode_message_content(content, "text");
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_decode_text_message_with_escaped_characters() {
        let content = r#"{"text":"Hello\nworld"}"#;
        let result = decode_message_content(content, "text");
        assert_eq!(result, "Hello\nworld");
    }

    #[test]
    fn test_decode_text_message_fallback_on_invalid_json() {
        let content = "plain text content";
        let result = decode_message_content(content, "text");
        assert_eq!(result, "plain text content");
    }

    #[test]
    fn test_decode_text_message_missing_text_field() {
        let content = r#"{"other":"value"}"#;
        let result = decode_message_content(content, "text");
        assert_eq!(result, content);
    }

    #[test]
    fn test_decode_text_message_empty_text_field() {
        let content = r#"{"text":""}"#;
        let result = decode_message_content(content, "text");
        assert_eq!(result, "");
    }

    #[test]
    fn test_decode_non_text_message_returns_content() {
        let content = "some content";
        let result = decode_message_content(content, "image");
        assert_eq!(result, "some content");
    }

    #[test]
    fn test_decode_non_text_message_with_json() {
        let content = r#"{"image_key":"img_v1_xxx"}"#;
        let result = decode_message_content(content, "image");
        assert_eq!(result, content);
    }

    #[test]
    fn test_encode_text_message() {
        let json = encode_text_message("Hello world");
        assert!(json.contains("\"text\":\"Hello world\""));
    }

    #[test]
    fn test_encode_text_message_empty() {
        let json = encode_text_message("");
        assert!(json.contains("\"text\":\"\""));
    }

    #[test]
    fn test_encode_text_message_with_newline() {
        let json = encode_text_message("line1\nline2");
        assert!(json.contains("line1\\nline2"));
    }
}

// Note: ChannelMessage does not implement serde::Deserialize, so we cannot test
// JSON deserialization directly. The struct is only used internally for message handling.

#[cfg(test)]
mod lark_api_error_tests {
    use ntd::feishu::sdk::error::LarkAPIError;

    #[test]
    fn test_lark_api_error_display_io() {
        let err = LarkAPIError::IOErr("connection refused".to_string());
        let display = format!("{}", err);
        assert!(display.contains("IO error"));
        assert!(display.contains("connection refused"));
    }

    #[test]
    fn test_lark_api_error_display_illegal_param() {
        let err = LarkAPIError::IllegalParamError("missing token".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Invalid parameter"));
        assert!(display.contains("missing token"));
    }

    #[test]
    fn test_lark_api_error_display_deserialize() {
        let err = LarkAPIError::DeserializeError("invalid json".to_string());
        let display = format!("{}", err);
        assert!(display.contains("JSON deserialization error"));
    }

    #[test]
    fn test_lark_api_error_display_request() {
        let err = LarkAPIError::RequestError("timeout".to_string());
        let display = format!("{}", err);
        assert!(display.contains("HTTP request failed"));
    }

    #[test]
    fn test_lark_api_error_display_url_parse() {
        let err = LarkAPIError::UrlParseError("invalid url".to_string());
        let display = format!("{}", err);
        assert!(display.contains("URL parse error"));
    }

    #[test]
    fn test_lark_api_error_display_api() {
        let err = LarkAPIError::ApiError {
            code: 99991401,
            message: "invalid access_token".to_string(),
            request_id: Some("req_123".to_string()),
        };
        let display = format!("{}", err);
        assert!(display.contains("invalid access_token"));
        assert!(display.contains("99991401"));
        assert!(display.contains("req_123"));
    }

    #[test]
    fn test_lark_api_error_display_missing_token() {
        let err = LarkAPIError::MissingAccessToken;
        let display = format!("{}", err);
        assert!(display.contains("Missing access token"));
    }

    #[test]
    fn test_lark_api_error_display_bad_request() {
        let err = LarkAPIError::BadRequest("malformed request".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Bad request"));
        assert!(display.contains("malformed request"));
    }

    #[test]
    fn test_lark_api_error_display_data_error() {
        let err = LarkAPIError::DataError("data not found".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Data error"));
        assert!(display.contains("data not found"));
    }

    #[test]
    fn test_lark_api_error_display_api_error_variant() {
        let err = LarkAPIError::APIError {
            code: 99991661,
            msg: "permission denied".to_string(),
            error: Some("access_denied".to_string()),
        };
        let display = format!("{}", err);
        assert!(display.contains("permission denied"));
        assert!(display.contains("99991661"));
    }

    #[test]
    fn test_lark_api_error_from_io_error() {
        use std::io;
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let lark_err: LarkAPIError = io_err.into();
        match lark_err {
            LarkAPIError::IOErr(msg) => assert!(msg.contains("file not found")),
            _ => panic!("expected IOErr"),
        }
    }

    #[test]
    fn test_lark_api_error_from_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let lark_err: LarkAPIError = json_err.into();
        match lark_err {
            LarkAPIError::DeserializeError(_) => {},
            _ => panic!("expected DeserializeError"),
        }
    }

    #[test]
    fn test_lark_api_error_debug() {
        let err = LarkAPIError::MissingAccessToken;
        let debug = format!("{:?}", err);
        assert!(debug.contains("MissingAccessToken"));
    }
}

#[cfg(test)]
mod pending_message_tests {
    use ntd::services::message_debounce::PendingMessage;
    use std::collections::HashMap;

    #[test]
    fn test_pending_message_creation() {
        let msg = PendingMessage {
            bot_id: 1,
            chat_id: "chat_123".to_string(),
            chat_type: "p2p".to_string(),
            sender: "user_456".to_string(),
            content: "Hello".to_string(),
            todo_id: 10,
            todo_prompt: "Do something".to_string(),
            executor: Some("kimi".to_string()),
            trigger_type: "feishu".to_string(),
            params: None,
            message_id: Some("msg_789".to_string()),
        };

        assert_eq!(msg.bot_id, 1);
        assert_eq!(msg.chat_id, "chat_123");
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.todo_id, 10);
        assert_eq!(msg.executor, Some("kimi".to_string()));
    }

    #[test]
    fn test_pending_message_with_params() {
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());

        let msg = PendingMessage {
            bot_id: 1,
            chat_id: "chat_123".to_string(),
            chat_type: "group".to_string(),
            sender: "user_456".to_string(),
            content: "Hello".to_string(),
            todo_id: 10,
            todo_prompt: "Do something".to_string(),
            executor: None,
            trigger_type: "feishu".to_string(),
            params: Some(params),
            message_id: None,
        };

        assert!(msg.params.is_some());
        assert_eq!(msg.params.as_ref().unwrap().get("key"), Some(&"value".to_string()));
    }
}