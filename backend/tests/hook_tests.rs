//! Tests for the inline-hook model: `HookTrigger`, `HookContext`, `TodoHooks`,
//! `TodoHookItem`. Hooks now live as a JSON array on each todo's `hooks`
//! column — there are no rules, no filters, no global defaults, no override
//! modes. Cycle protection is asserted via the `chain` field on `HookContext`.

#[cfg(test)]
mod hook_trigger_tests {
    use ntd::hooks::HookTrigger;
    use ntd::models::TodoStatus;

    #[test]
    fn as_str_covers_all_variants() {
        assert_eq!(HookTrigger::BeforeCreate.as_str(), "before_create");
        assert_eq!(HookTrigger::AfterCreate.as_str(), "after_create");
        assert_eq!(HookTrigger::StateChangedToPending.as_str(), "state_changed_to_pending");
        assert_eq!(HookTrigger::StateChangedToInProgress.as_str(), "state_changed_to_in_progress");
        assert_eq!(HookTrigger::StateChangedToCompleted.as_str(), "state_changed_to_completed");
        assert_eq!(HookTrigger::StateChangedToFailed.as_str(), "state_changed_to_failed");
        assert_eq!(HookTrigger::BeforeDelete.as_str(), "before_delete");
        assert_eq!(HookTrigger::AfterDelete.as_str(), "after_delete");
    }

    #[test]
    fn from_str_round_trips_and_is_case_sensitive() {
        for t in [
            HookTrigger::BeforeCreate,
            HookTrigger::AfterCreate,
            HookTrigger::StateChangedToPending,
            HookTrigger::StateChangedToInProgress,
            HookTrigger::StateChangedToCompleted,
            HookTrigger::StateChangedToFailed,
            HookTrigger::BeforeDelete,
            HookTrigger::AfterDelete,
        ] {
            assert_eq!(HookTrigger::from_str(t.as_str()), Some(t));
        }
        assert_eq!(HookTrigger::from_str("invalid"), None);
        assert_eq!(HookTrigger::from_str(""), None);
        assert_eq!(HookTrigger::from_str("BEFORE_CREATE"), None);
    }

    #[test]
    fn is_sync_only_for_before_lifecycle() {
        assert!(HookTrigger::BeforeCreate.is_sync());
        assert!(HookTrigger::BeforeDelete.is_sync());

        assert!(!HookTrigger::AfterCreate.is_sync());
        assert!(!HookTrigger::AfterDelete.is_sync());
        assert!(!HookTrigger::StateChangedToPending.is_sync());
        assert!(!HookTrigger::StateChangedToInProgress.is_sync());
        assert!(!HookTrigger::StateChangedToCompleted.is_sync());
        assert!(!HookTrigger::StateChangedToFailed.is_sync());
    }

    #[test]
    fn for_target_status_maps_each_observable_state() {
        assert_eq!(
            HookTrigger::for_target_status(TodoStatus::Pending),
            Some(HookTrigger::StateChangedToPending),
        );
        assert_eq!(
            HookTrigger::for_target_status(TodoStatus::InProgress),
            Some(HookTrigger::StateChangedToInProgress),
        );
        assert_eq!(
            HookTrigger::for_target_status(TodoStatus::Running),
            Some(HookTrigger::StateChangedToInProgress),
        );
        assert_eq!(
            HookTrigger::for_target_status(TodoStatus::Completed),
            Some(HookTrigger::StateChangedToCompleted),
        );
        assert_eq!(
            HookTrigger::for_target_status(TodoStatus::Failed),
            Some(HookTrigger::StateChangedToFailed),
        );
        // Cancelled is intentionally not observable — UI cancels are noise.
        assert_eq!(HookTrigger::for_target_status(TodoStatus::Cancelled), None);
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", HookTrigger::AfterCreate), "after_create");
    }
}

#[cfg(test)]
mod todo_hook_item_tests {
    use ntd::hooks::{HookTrigger, TodoHookItem};

    #[test]
    fn deserialize_minimal_defaults_enabled_true() {
        let json = r#"{
            "id": 1,
            "trigger": "after_create",
            "target_todo_id": 42
        }"#;
        let item: TodoHookItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, 1);
        assert_eq!(item.trigger, HookTrigger::AfterCreate);
        assert_eq!(item.target_todo_id, 42);
        assert!(!item.skip_if_missing);
        assert!(item.enabled); // serde default
    }

    #[test]
    fn deserialize_full_round_trip() {
        let item = TodoHookItem {
            id: 7,
            trigger: HookTrigger::StateChangedToCompleted,
            target_todo_id: 99,
            skip_if_missing: true,
            enabled: false,
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: TodoHookItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, item.id);
        assert_eq!(decoded.trigger, item.trigger);
        assert_eq!(decoded.target_todo_id, item.target_todo_id);
        assert_eq!(decoded.skip_if_missing, item.skip_if_missing);
        assert_eq!(decoded.enabled, item.enabled);
    }

    #[test]
    fn deserialize_drops_legacy_prompt_template_field() {
        // Old hook rows in the DB (from before this change) may still carry
        // a `prompt_template` key. The deserializer should ignore it instead
        // of failing — the field was removed.
        let json = r#"{
            "id": 1,
            "trigger": "after_create",
            "target_todo_id": 42,
            "prompt_template": "legacy value to be ignored"
        }"#;
        let item: TodoHookItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, 1);
        assert_eq!(item.target_todo_id, 42);
    }
}

#[cfg(test)]
mod todo_hooks_tests {
    use ntd::hooks::{HookTrigger, TodoHookItem, TodoHooks};

    fn item(id: i64, trigger: HookTrigger, target: i64, enabled: bool) -> TodoHookItem {
        TodoHookItem {
            id,
            trigger,
            target_todo_id: target,
            skip_if_missing: false,
            enabled,
        }
    }

    #[test]
    fn parse_none_returns_default() {
        let parsed = TodoHooks::parse(None);
        assert!(parsed.items.is_empty());
    }

    #[test]
    fn parse_empty_string_returns_default() {
        let parsed = TodoHooks::parse(Some(""));
        assert!(parsed.items.is_empty());
    }

    #[test]
    fn parse_malformed_json_returns_default_without_panicking() {
        let parsed = TodoHooks::parse(Some("not json"));
        assert!(parsed.items.is_empty());
    }

    #[test]
    fn parse_valid_json_round_trips() {
        let source = TodoHooks {
            items: vec![item(1, HookTrigger::AfterCreate, 5, true)],
        };
        let json = serde_json::to_string(&source).unwrap();
        let parsed = TodoHooks::parse(Some(&json));
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].id, 1);
        assert_eq!(parsed.items[0].target_todo_id, 5);
    }

    #[test]
    fn matching_filters_by_trigger_and_enabled() {
        let hooks = TodoHooks {
            items: vec![
                item(1, HookTrigger::AfterCreate, 5, true),
                item(2, HookTrigger::AfterCreate, 6, false), // disabled
                item(3, HookTrigger::StateChangedToCompleted, 7, true), // wrong trigger
                item(4, HookTrigger::AfterCreate, 8, true),
            ],
        };
        let matched: Vec<i64> = hooks
            .matching(HookTrigger::AfterCreate)
            .map(|i| i.id)
            .collect();
        assert_eq!(matched, vec![1, 4]);
    }

    #[test]
    fn matching_empty_when_no_trigger_match() {
        let hooks = TodoHooks {
            items: vec![item(1, HookTrigger::AfterCreate, 5, true)],
        };
        assert_eq!(
            hooks.matching(HookTrigger::StateChangedToFailed).count(),
            0
        );
    }
}

#[cfg(test)]
mod hook_context_tests {
    use ntd::hooks::{HookContext, HookTrigger};
    use ntd::models::TodoStatus;

    #[test]
    fn for_create_carries_chain_and_default_trigger() {
        let ctx = HookContext::for_create(
            "New Todo".to_string(),
            Some("claude".to_string()),
            Some("/workspace".to_string()),
            vec![10, 11],
        );
        assert_eq!(ctx.todo_id, None);
        assert_eq!(ctx.todo_title, "New Todo");
        assert_eq!(ctx.old_status, None);
        assert_eq!(ctx.new_status.as_deref(), Some("pending"));
        assert_eq!(ctx.executor.as_deref(), Some("claude"));
        assert_eq!(ctx.workspace.as_deref(), Some("/workspace"));
        assert_eq!(ctx.trigger, HookTrigger::BeforeCreate);
        assert_eq!(ctx.chain, vec![10, 11]);
        assert!(!ctx.trigger_time.is_empty());
    }

    #[test]
    fn for_create_after_attaches_todo_id_and_trigger() {
        let ctx = HookContext::for_create_after(
            55,
            "Title".to_string(),
            None,
            None,
            vec![55],
        );
        assert_eq!(ctx.todo_id, Some(55));
        assert_eq!(ctx.trigger, HookTrigger::AfterCreate);
        assert_eq!(ctx.chain, vec![55]);
    }

    #[test]
    fn for_state_change_maps_each_observable_status() {
        let cases = [
            (TodoStatus::Pending, HookTrigger::StateChangedToPending),
            (TodoStatus::InProgress, HookTrigger::StateChangedToInProgress),
            (TodoStatus::Running, HookTrigger::StateChangedToInProgress),
            (TodoStatus::Completed, HookTrigger::StateChangedToCompleted),
            (TodoStatus::Failed, HookTrigger::StateChangedToFailed),
        ];
        for (status, expected) in cases {
            let ctx = HookContext::for_state_change(
                1,
                "x".to_string(),
                TodoStatus::Pending,
                status,
                None,
                None,
                vec![],
            )
            .unwrap_or_else(|| panic!("status {:?} should map to a trigger", status));
            assert_eq!(ctx.trigger, expected);
            assert_eq!(ctx.new_status.as_deref(), Some(status.to_string().as_str()));
        }
    }

    #[test]
    fn for_state_change_returns_none_for_cancelled() {
        let ctx = HookContext::for_state_change(
            1,
            "x".to_string(),
            TodoStatus::InProgress,
            TodoStatus::Cancelled,
            None,
            None,
            vec![],
        );
        assert!(ctx.is_none());
    }

    #[test]
    fn for_delete_carries_old_status_and_trigger() {
        let ctx = HookContext::for_delete(
            99,
            "Deleted".to_string(),
            TodoStatus::Running,
            Some("claude".to_string()),
            None,
            vec![99],
        );
        assert_eq!(ctx.todo_id, Some(99));
        assert_eq!(ctx.old_status.as_deref(), Some("running"));
        assert_eq!(ctx.new_status, None);
        assert_eq!(ctx.trigger, HookTrigger::BeforeDelete);
        assert_eq!(ctx.chain, vec![99]);
    }

    #[test]
    fn for_delete_after_has_after_delete_trigger() {
        let ctx = HookContext::for_delete_after(
            100,
            "Gone".to_string(),
            TodoStatus::Completed,
            None,
            None,
            vec![100],
        );
        assert_eq!(ctx.trigger, HookTrigger::AfterDelete);
    }

    #[test]
    fn to_params_includes_chain_as_comma_string() {
        let ctx = HookContext::for_create_after(
            10,
            "Title".to_string(),
            None,
            None,
            vec![1, 2, 3],
        );
        let params = ctx.to_params();
        assert_eq!(params.get("chain").map(|s| s.as_str()), Some("1,2,3"));
        assert_eq!(params.get("todo_id").map(|s| s.as_str()), Some("10"));
        assert_eq!(params.get("todo_title").map(|s| s.as_str()), Some("Title"));
        assert_eq!(
            params.get("trigger").map(|s| s.as_str()),
            Some("after_create")
        );
    }

    #[test]
    fn to_params_chain_empty_when_no_visits() {
        let ctx = HookContext::for_create(
            "T".to_string(),
            None,
            None,
            vec![],
        );
        let params = ctx.to_params();
        assert_eq!(params.get("chain").map(|s| s.as_str()), Some(""));
    }
}

#[cfg(test)]
mod hook_dispatch_tests {
    use ntd::hooks::{TodoHookItem, HookTrigger};
    use ntd::models::{Todo, TodoStatus};

    fn todo(id: i64, title: &str, prompt: &str, status: TodoStatus) -> Todo {
        Todo {
            id,
            title: title.to_string(),
            prompt: prompt.to_string(),
            status,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            tag_ids: vec![],
            executor: Some("claudecode".to_string()),
            scheduler_enabled: false,
            scheduler_config: None,
            scheduler_timezone: None,
            scheduler_next_run_at: None,
            task_id: None,
            workspace: Some("/tmp/work".to_string()),
            worktree_enabled: false,
            hooks: vec![],
        }
    }

    #[test]
    fn build_hook_message_uses_result_when_provided() {
        // When the source's executor has produced output, that result is
        // what flows into the target's `{{message}}` placeholder — not the
        // source's own prompt. The result IS the answer; the prompt is just
        // the question.
        let source = todo(42, "笑话", "讲个程序员笑话", TodoStatus::Completed);
        let target = todo(
            7,
            "评论生成器",
            "请基于以下内容写评论：\n{{message}}",
            TodoStatus::Pending,
        );

        let result = "一个关于goto的程序员笑话...";
        let message = ntd::hooks::build_hook_message(&source, Some(result));
        let params = std::collections::HashMap::from([("message".to_string(), message)]);
        let rendered = ntd::models::replace_placeholders(&target.prompt, &params);
        assert_eq!(
            rendered,
            "请基于以下内容写评论：\n一个关于goto的程序员笑话..."
        );
    }

    #[test]
    fn build_hook_message_falls_back_to_source_prompt_without_result() {
        // No execution result yet (e.g., for `after_create` where the source
        // just came into being, or a manual status change with no prior
        // run). Fall back to the source's prompt so the target still gets
        // useful context.
        let source = todo(42, "笑话", "讲个程序员笑话", TodoStatus::Completed);
        let target = todo(
            7,
            "评论生成器",
            "请基于以下内容写评论：\n{{message}}",
            TodoStatus::Pending,
        );

        let message = ntd::hooks::build_hook_message(&source, None);
        assert_eq!(message, "讲个程序员笑话");
        let params = std::collections::HashMap::from([("message".to_string(), message)]);
        let rendered = ntd::models::replace_placeholders(&target.prompt, &params);
        assert_eq!(
            rendered,
            "请基于以下内容写评论：\n讲个程序员笑话"
        );
    }

    #[test]
    fn todo_hook_item_no_prompt_template_field() {
        // The hook item carries only: id, trigger, target_todo_id, skip_if_missing, enabled.
        // The "what to send" is configured entirely on the target todo's own prompt.
        let item = TodoHookItem {
            id: 1,
            trigger: HookTrigger::StateChangedToCompleted,
            target_todo_id: 7,
            skip_if_missing: true,
            enabled: true,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(!json.contains("prompt_template"), "prompt_template must not be in the wire format: {}", json);
    }
}
