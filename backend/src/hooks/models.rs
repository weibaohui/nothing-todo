use serde::{Deserialize, Serialize};
use crate::models::TodoStatus;

/// Hook trigger types.
///
/// Triggers are tied to todo lifecycle events. State-change triggers are
/// per-target-state: each fires when a todo transitions INTO that status.
/// There is intentionally no "before status change" trigger — hooks observe
/// state changes, they don't gate them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTrigger {
    BeforeCreate,
    AfterCreate,
    StateChangedToPending,
    StateChangedToInProgress,
    StateChangedToCompleted,
    StateChangedToFailed,
    BeforeDelete,
    AfterDelete,
}

impl HookTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BeforeCreate => "before_create",
            Self::AfterCreate => "after_create",
            Self::StateChangedToPending => "state_changed_to_pending",
            Self::StateChangedToInProgress => "state_changed_to_in_progress",
            Self::StateChangedToCompleted => "state_changed_to_completed",
            Self::StateChangedToFailed => "state_changed_to_failed",
            Self::BeforeDelete => "before_delete",
            Self::AfterDelete => "after_delete",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "before_create" => Some(Self::BeforeCreate),
            "after_create" => Some(Self::AfterCreate),
            "state_changed_to_pending" => Some(Self::StateChangedToPending),
            "state_changed_to_in_progress" => Some(Self::StateChangedToInProgress),
            "state_changed_to_completed" => Some(Self::StateChangedToCompleted),
            "state_changed_to_failed" => Some(Self::StateChangedToFailed),
            "before_delete" => Some(Self::BeforeDelete),
            "after_delete" => Some(Self::AfterDelete),
            _ => None,
        }
    }

    /// Whether this trigger runs synchronously and can block the operation.
    /// Only `before_*` lifecycle gates are sync; state-change triggers observe.
    pub fn is_sync(&self) -> bool {
        matches!(self, Self::BeforeCreate | Self::BeforeDelete)
    }

    /// Map a target `TodoStatus` to its corresponding state-change trigger.
    /// Returns `None` for statuses without a dedicated trigger (e.g. `cancelled`)
    /// so callers can decide whether to fire any hook at all.
    pub fn for_target_status(status: TodoStatus) -> Option<Self> {
        match status {
            TodoStatus::Pending => Some(Self::StateChangedToPending),
            TodoStatus::InProgress | TodoStatus::Running => Some(Self::StateChangedToInProgress),
            TodoStatus::Completed => Some(Self::StateChangedToCompleted),
            TodoStatus::Failed => Some(Self::StateChangedToFailed),
            TodoStatus::Cancelled => None,
        }
    }
}

impl std::fmt::Display for HookTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single hook attached to a todo. When the parent todo emits a matching
/// `trigger`, the service starts the `target_todo` with the source todo's
/// `prompt` injected as the `{{message}}` placeholder inside the target's
/// own prompt — same mental model as the manual "execute with args" flow.
///
/// Hooks are stored inline on the todo row (one JSON column) — there is no
/// global hook rule library, no per-todo override mode, and no global default
/// list. Each todo owns its own hooks and is the only place that can fire them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoHookItem {
    /// Client-generated stable id, used to identify the item in edit/delete UI
    /// before the row has been persisted.
    pub id: i64,
    pub trigger: HookTrigger,
    pub target_todo_id: i64,
    #[serde(default)]
    pub skip_if_missing: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Wrapper for the `todos.hooks` JSON column.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TodoHooks {
    #[serde(default)]
    pub items: Vec<TodoHookItem>,
}

impl TodoHooks {
    /// Parse the JSON string stored in the `todos.hooks` column.
    /// Returns `TodoHooks::default()` when the column is `None` or empty or
    /// contains malformed JSON — we never want a bad value to break todo
    /// loading.
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(s) = raw else { return Self::default() };
        if s.is_empty() {
            return Self::default();
        }
        serde_json::from_str(s).unwrap_or_default()
    }

    /// Filter to enabled items whose trigger matches.
    pub fn matching(&self, trigger: HookTrigger) -> impl Iterator<Item = &TodoHookItem> {
        self.items
            .iter()
            .filter(move |item| item.enabled && item.trigger == trigger)
    }
}

/// Build the `message` payload that a hook delivers to the target todo's
/// executor. The target todo's own `prompt` is the template (it may contain
/// a `{{message}}` placeholder where this value lands), so by the time the
/// executor sees the final message it looks like:
///
/// ```text
/// <target todo's prompt, with {{message}} replaced>
/// ```
///
/// The `{{message}}` value is:
/// - The source todo's latest successful execution `result` when one exists
///   — i.e., what the previous executor actually produced. This is the
///   primary use case: "A ran, take A's output and feed it to B."
/// - Otherwise the source's `prompt` — what the user originally asked the
///   source todo. Used when the hook fires before the source has run (e.g.,
///   `after_create`) or when the source has never been executed.
///
/// This mirrors the manual "execute with args" flow: the user edits the
/// target todo's prompt with `{{message}}` where the source context should
/// land, hooks automatically supply the value. No new template syntax to
/// learn.
pub fn build_hook_message(
    source: &crate::models::Todo,
    result: Option<&str>,
) -> String {
    result
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| source.prompt.clone())
}

/// Hook execution context (data passed through the chain).
///
/// `chain` records every todo id already visited on the current dispatch path
/// (including the source todo at position 0). A hook that would re-visit any
/// id in `chain` is rejected, which breaks the only cycle the system can
/// produce: A → B → A.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub todo_id: Option<i64>,
    pub todo_title: String,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub executor: Option<String>,
    pub workspace: Option<String>,
    pub task_id: Option<String>,
    pub trigger_time: String,
    pub trigger: HookTrigger,
    #[serde(default)]
    pub chain: Vec<i64>,
}

impl HookContext {
    /// Convert the context to a `HashMap<String, String>` for placeholder substitution
    /// in target todo prompts.
    pub fn to_params(&self) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();
        params.insert(
            "todo_id".to_string(),
            self.todo_id.map(|id| id.to_string()).unwrap_or_default(),
        );
        params.insert("todo_title".to_string(), self.todo_title.clone());
        params.insert("old_status".to_string(), self.old_status.clone().unwrap_or_default());
        params.insert("new_status".to_string(), self.new_status.clone().unwrap_or_default());
        params.insert("executor".to_string(), self.executor.clone().unwrap_or_default());
        params.insert("workspace".to_string(), self.workspace.clone().unwrap_or_default());
        params.insert("task_id".to_string(), self.task_id.clone().unwrap_or_default());
        params.insert("trigger_time".to_string(), self.trigger_time.clone());
        params.insert("trigger".to_string(), self.trigger.to_string());
        params.insert("chain".to_string(), {
            let parts: Vec<String> = self.chain.iter().map(|id| id.to_string()).collect();
            parts.join(",")
        });
        params
    }

    pub fn for_create(
        todo_title: String,
        executor: Option<String>,
        workspace: Option<String>,
        chain: Vec<i64>,
    ) -> Self {
        Self {
            todo_id: None,
            todo_title,
            old_status: None,
            new_status: Some("pending".to_string()),
            executor,
            workspace,
            task_id: None,
            trigger_time: crate::models::utc_timestamp(),
            trigger: HookTrigger::BeforeCreate,
            chain,
        }
    }

    pub fn for_create_after(
        todo_id: i64,
        todo_title: String,
        executor: Option<String>,
        workspace: Option<String>,
        chain: Vec<i64>,
    ) -> Self {
        Self {
            todo_id: Some(todo_id),
            todo_title,
            old_status: None,
            new_status: Some("pending".to_string()),
            executor,
            workspace,
            task_id: None,
            trigger_time: crate::models::utc_timestamp(),
            trigger: HookTrigger::AfterCreate,
            chain,
        }
    }

    /// Build a state-change context for a todo transitioning to `new_status`.
    /// The trigger is selected from `HookTrigger::for_target_status(new_status)`.
    /// Returns `None` when the target status has no dedicated trigger
    /// (e.g. `cancelled`).
    pub fn for_state_change(
        todo_id: i64,
        todo_title: String,
        old_status: TodoStatus,
        new_status: TodoStatus,
        executor: Option<String>,
        workspace: Option<String>,
        chain: Vec<i64>,
    ) -> Option<Self> {
        let trigger = HookTrigger::for_target_status(new_status)?;
        Some(Self {
            todo_id: Some(todo_id),
            todo_title,
            old_status: Some(old_status.to_string()),
            new_status: Some(new_status.to_string()),
            executor,
            workspace,
            task_id: None,
            trigger_time: crate::models::utc_timestamp(),
            trigger,
            chain,
        })
    }

    pub fn for_delete(
        todo_id: i64,
        todo_title: String,
        status: TodoStatus,
        executor: Option<String>,
        workspace: Option<String>,
        chain: Vec<i64>,
    ) -> Self {
        Self {
            todo_id: Some(todo_id),
            todo_title,
            old_status: Some(status.to_string()),
            new_status: None,
            executor,
            workspace,
            task_id: None,
            trigger_time: crate::models::utc_timestamp(),
            trigger: HookTrigger::BeforeDelete,
            chain,
        }
    }

    pub fn for_delete_after(
        todo_id: i64,
        todo_title: String,
        status: TodoStatus,
        executor: Option<String>,
        workspace: Option<String>,
        chain: Vec<i64>,
    ) -> Self {
        Self {
            todo_id: Some(todo_id),
            todo_title,
            old_status: Some(status.to_string()),
            new_status: None,
            executor,
            workspace,
            task_id: None,
            trigger_time: crate::models::utc_timestamp(),
            trigger: HookTrigger::AfterDelete,
            chain,
        }
    }
}
