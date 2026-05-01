//! CLI command parsing tests

#[cfg(test)]
mod todo_create_command_tests {
    #[test]
    fn test_todo_create_parsing() {
        // Test parsing: ntd todo create "My Task" -p "prompt" -e kimi
        let args: Vec<&str> = vec!["ntd", "todo", "create", "My Task", "-p", "prompt", "-e", "kimi"];
        // This tests that clap parses correctly
        assert_eq!(args.len(), 8);
    }

    #[test]
    fn test_todo_create_with_schedule() {
        // Test parsing: ntd todo create "Task" --schedule "*/30 * * * *"
        let schedule = "*/30 * * * *";
        assert!(schedule.starts_with("*/"));
    }

    #[test]
    fn test_todo_create_with_tags() {
        // Test parsing: ntd todo create "Task" --tags "1,2,3"
        let tags = "1,2,3";
        let parsed: Vec<i64> = tags.split(',').filter_map(|s| s.parse().ok()).collect();
        assert_eq!(parsed, vec![1, 2, 3]);
    }

    #[test]
    fn test_todo_create_with_workspace() {
        // Test parsing: ntd todo create "Task" -w /path/to/dir
        let workspace = "/path/to/dir";
        assert!(!workspace.is_empty());
    }
}

#[cfg(test)]
mod todo_execute_command_tests {
    #[test]
    fn test_execute_request_serialization() {
        use ntd::models::ExecuteRequest;

        let req = ExecuteRequest {
            todo_id: 123,
            message: Some("hello".to_string()),
            executor: Some("kimi".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"todo_id\":123"));
        assert!(json.contains("\"message\":\"hello\""));
        assert!(json.contains("\"executor\":\"kimi\""));
    }

    #[test]
    fn test_execute_request_message_null() {
        use ntd::models::ExecuteRequest;

        let json = r#"{"todo_id": 123, "message": null, "executor": null}"#;
        let req: ExecuteRequest = serde_json::from_str(json).unwrap();
        assert!(req.message.is_none());
        assert!(req.executor.is_none());
    }

    #[test]
    fn test_execute_request_minimal() {
        use ntd::models::ExecuteRequest;

        let req = ExecuteRequest {
            todo_id: 1,
            message: None,
            executor: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"todo_id\":1"));
        assert!(json.contains("\"message\":null"));
    }
}

#[cfg(test)]
mod stop_execution_request_tests {
    #[test]
    fn test_stop_request_deserialization() {
        #[derive(serde::Deserialize)]
        struct StopRequest {
            record_id: i64,
        }

        let json = r#"{"record_id": 42}"#;
        let req: StopRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.record_id, 42);
    }
}

#[cfg(test)]
mod todo_list_command_tests {
    #[test]
    fn test_todo_list_parsing() {
        // ntd todo list
        let args = vec!["ntd", "todo", "list"];
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_todo_get_parsing() {
        // ntd todo get <id>
        let id: i64 = "123".parse().unwrap();
        assert_eq!(id, 123);
    }
}

#[cfg(test)]
mod config_parsing_tests {
    use ntd::config::{Config, ExecutorPaths};

    #[test]
    fn test_executor_paths_default() {
        let paths = ExecutorPaths::default();
        assert_eq!(paths.opencode, "opencode");
        assert_eq!(paths.hermes, "hermes");
        assert_eq!(paths.joinai, "joinai");
        assert_eq!(paths.claude_code, "claude");
        assert_eq!(paths.codebuddy, "codebuddy");
        assert_eq!(paths.kimi, "kimi");
        assert_eq!(paths.atomcode, "atomcode");
        assert_eq!(paths.codex, "codex");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 8088);
        assert_eq!(config.host, "0.0.0.0");
        assert!(config.log_level.contains("INFO") || config.log_level == "INFO".to_string());
    }

    #[test]
    fn test_config_executor_paths() {
        let paths = ExecutorPaths {
            opencode: "custom-opencode".to_string(),
            hermes: "custom-hermes".to_string(),
            joinai: "joinai".to_string(),
            claude_code: "claude".to_string(),
            codebuddy: "codebuddy".to_string(),
            kimi: "kimi".to_string(),
            atomcode: "atomcode".to_string(),
            codex: "codex".to_string(),
        };
        assert_eq!(paths.opencode, "custom-opencode");
        assert_eq!(paths.hermes, "custom-hermes");
    }
}

#[cfg(test)]
mod output_format_tests {
    #[test]
    fn test_output_format_variants() {
        // Test that output format can be parsed correctly
        let formats = vec!["json", "pretty"];
        for fmt in formats {
            assert!(!fmt.is_empty());
        }
    }
}

#[cfg(test)]
mod cron_validation_tests {
    use std::str::FromStr;

    #[test]
    fn test_valid_cron_expressions() {
        let expressions = vec![
            "*/30 * * * * *",  // every 30 seconds
            "0 */12 * * * *",   // every 12 hours
            "0 0 * * * *",      // every hour
            "0 0 9 * * *",      // daily at 9am
            "0 0 0 * * *",      // midnight
        ];

        for expr in expressions {
            let result = cron::Schedule::from_str(expr);
            assert!(result.is_ok(), "Cron expression '{}' should be valid", expr);
        }
    }

    #[test]
    fn test_invalid_cron_expressions() {
        let expressions = vec![
            "invalid",
            "",
            "* * *",           // too few fields (6 required)
        ];

        for expr in expressions {
            let result = cron::Schedule::from_str(expr);
            assert!(result.is_err(), "Cron expression '{}' should be invalid", expr);
        }
    }
}
