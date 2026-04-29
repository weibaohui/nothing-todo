use crate::models::{ParsedLogEntry, TodoItem};

const TODO_TOOL_NAMES: &[&str] = &["todowrite", "writetodo", "todo_write", "write_todo"];

fn is_todo_tool_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    TODO_TOOL_NAMES.iter().any(|&n| n == lower)
}

/// Try to extract a todo list from a parsed log entry.
/// Returns Some(Vec<TodoItem>) if the entry represents a todo-writing tool call.
pub fn try_extract_todo_progress(entry: &ParsedLogEntry) -> Option<Vec<TodoItem>> {
    let tool_name = entry.tool_name.as_ref()?;
    if !is_todo_tool_name(tool_name) {
        return None;
    }
    let input_json = entry.tool_input_json.as_ref()?;
    let input: serde_json::Value = serde_json::from_str(input_json).ok()?;
    extract_todos_from_input(&input)
}

fn extract_todos_from_input(input: &serde_json::Value) -> Option<Vec<TodoItem>> {
    // Try "todos" array (Claude Code TodoWrite format)
    if let Some(todos) = input.get("todos").and_then(|v| v.as_array()) {
        let items: Vec<TodoItem> = todos.iter().filter_map(parse_todo_item).collect();
        if !items.is_empty() {
            return Some(items);
        }
    }
    // Try "items" array (alternative format)
    if let Some(items) = input.get("items").and_then(|v| v.as_array()) {
        let items: Vec<TodoItem> = items.iter().filter_map(parse_todo_item).collect();
        if !items.is_empty() {
            return Some(items);
        }
    }
    None
}

fn parse_todo_item(v: &serde_json::Value) -> Option<TodoItem> {
    let content = v
        .get("content")
        .or_else(|| v.get("title"))
        .or_else(|| v.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if content.is_empty() {
        return None;
    }
    Some(TodoItem {
        id: v.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        content,
        status: v
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string(),
    })
}
