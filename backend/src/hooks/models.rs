use serde::{Deserialize, Serialize};
use tracing::warn;
use crate::models::TodoStatus;

/// Hook trigger types.
///
/// The only triggers that exist are per-target-state: each fires when a todo
/// transitions INTO that status. There are intentionally no lifecycle gates
/// (`before_create` / `after_create` / `before_delete` / `after_delete`) —
/// hooks observe state changes, they don't gate lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTrigger {
    StateChangedToPending,
    StateChangedToInProgress,
    StateChangedToCompleted,
    StateChangedToFailed,
}

impl HookTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StateChangedToPending => "state_changed_to_pending",
            Self::StateChangedToInProgress => "state_changed_to_in_progress",
            Self::StateChangedToCompleted => "state_changed_to_completed",
            Self::StateChangedToFailed => "state_changed_to_failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "state_changed_to_pending" => Some(Self::StateChangedToPending),
            "state_changed_to_in_progress" => Some(Self::StateChangedToInProgress),
            "state_changed_to_completed" => Some(Self::StateChangedToCompleted),
            "state_changed_to_failed" => Some(Self::StateChangedToFailed),
            _ => None,
        }
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
    /// Optional rating gate (0-100). When set, the hook only fires if the
    /// source todo's most recent FINISHED execution record has a
    /// `rating >= min_rating`. `None` means no gate (always fire) — this
    /// preserves the original behaviour for any hook that doesn't opt in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_rating: Option<i32>,
    /// What to do when the latest finished record has no rating (NULL).
    /// Defaults to `Skip` (do not fire) — the conservative choice for users
    /// who explicitly enabled a rating gate: if you cared enough to set a
    /// threshold, an unrated result is probably not what you wanted to chain
    /// off of. `Pass` is the opt-in permissive mode for teams that want the
    /// gate to only block obviously-bad runs.
    #[serde(default = "default_unrated_policy")]
    pub unrated_policy: UnratedPolicy,
}

fn default_enabled() -> bool {
    true
}

fn default_unrated_policy() -> UnratedPolicy {
    UnratedPolicy::Skip
}

/// Policy applied by the rating gate when the latest finished record has
/// no rating set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UnratedPolicy {
    /// Do not fire the hook. This is the safe default: an unrated record
    /// can't be evaluated against `min_rating`, so we treat "no opinion" as
    /// "don't chain off of this".
    #[default]
    Skip,
    /// Treat the missing rating as if it had passed. Useful when you only
    /// want the gate to actively block runs you've explicitly scored low.
    Pass,
}

impl UnratedPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::Pass => "pass",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "skip" => Some(Self::Skip),
            "pass" => Some(Self::Pass),
            _ => None,
        }
    }
}

impl std::fmt::Display for UnratedPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
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
    ///
    /// Items whose `trigger` doesn't deserialize to a current `HookTrigger`
    /// variant are silently dropped. This keeps todo loading working after
    /// trigger types are removed: any rows written by an older build that
    /// still carry e.g. `before_create` simply contribute zero items.
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(s) = raw else { return Self::default() };
        if s.is_empty() {
            return Self::default();
        }
        let parsed: RawTodoHooks = serde_json::from_str(s).unwrap_or_default();
        Self {
            items: parsed
                .items
                .into_iter()
                .filter_map(|raw| {
                    let trigger = HookTrigger::from_str(&raw.trigger)?;
                    // Validate rating gate fields defensively: a malformed
                    // value (out-of-range or unknown policy) shouldn't take
                    // down the whole hook list, it just disables this item.
                    let min_rating = match raw.min_rating {
                        None => None,
                        Some(v) if (0..=100).contains(&v) => Some(v),
                        Some(v) => {
                            warn!(
                                "hook #{} has out-of-range min_rating {}, ignoring",
                                raw.id, v
                            );
                            None
                        }
                    };
                    let unrated_policy = raw
                        .unrated_policy
                        .as_deref()
                        .and_then(UnratedPolicy::from_str)
                        .unwrap_or_default();
                    Some(TodoHookItem {
                        id: raw.id,
                        trigger,
                        target_todo_id: raw.target_todo_id,
                        skip_if_missing: raw.skip_if_missing,
                        enabled: raw.enabled,
                        min_rating,
                        unrated_policy,
                    })
                })
                .collect(),
        }
    }

    /// Filter to enabled items whose trigger matches.
    pub fn matching(&self, trigger: HookTrigger) -> impl Iterator<Item = &TodoHookItem> {
        self.items
            .iter()
            .filter(move |item| item.enabled && item.trigger == trigger)
    }
}

/// Intermediate parse shape: the `trigger` is a free-form string so the
/// deserializer can capture values that no longer map to a `HookTrigger`
/// variant, letting `parse` drop them rather than failing the whole row.
#[derive(Default, Deserialize)]
struct RawTodoHooks {
    #[serde(default)]
    items: Vec<RawTodoHookItem>,
}

#[derive(Deserialize)]
struct RawTodoHookItem {
    id: i64,
    trigger: String,
    target_todo_id: i64,
    #[serde(default)]
    skip_if_missing: bool,
    #[serde(default = "default_enabled")]
    enabled: bool,
    /// Optional rating threshold (0-100). Older payloads won't have this
    /// field and serde's `default` keeps them working unchanged.
    #[serde(default)]
    min_rating: Option<i32>,
    /// Policy for when the latest record has no rating. Stored as a snake
    /// case string to match the on-disk format; unknown values fall back to
    /// the default policy at `parse` time.
    #[serde(default)]
    unrated_policy: Option<String>,
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
/// The `{{message}}` value is the source todo's latest successful execution
/// `result` — what the previous executor actually produced. This is the
/// primary use case: "A ran, take A's output and feed it to B." When the
/// source has no successful run yet (e.g., a state-change trigger fires
/// immediately on creation), the source's `prompt` is used as the fallback
/// so the target still gets useful context.
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
}
