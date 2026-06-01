/** Inline hook items attached to a todo. Hooks live in the `todos.hooks`
 *  column as a JSON array — there is no global hook library or rules engine.
 *
 *  When a hook fires, the target todo's own `prompt` is the template. The
 *  `{{message}}` placeholder is filled with the source todo's most recent
 *  successful execution `result` (what its executor actually produced) so
 *  the chain flows A's output into B. If the source has not run yet (e.g.
 *  `after_create`), the source's `prompt` is used as a fallback.
 *
 *  This matches the manual "execute with args" flow — the user writes the
 *  template by editing the target todo's prompt, hooks just supply the
 *  `{{message}}` value automatically. */

export type HookTrigger =
  | 'before_create'
  | 'after_create'
  | 'state_changed_to_pending'
  | 'state_changed_to_in_progress'
  | 'state_changed_to_completed'
  | 'state_changed_to_failed'
  | 'before_delete'
  | 'after_delete';

export interface TodoHookItem {
  id: number;
  trigger: HookTrigger;
  target_todo_id: number;
  skip_if_missing?: boolean;
  enabled: boolean;
}

export const HOOK_TRIGGERS: ReadonlyArray<{ value: HookTrigger; label: string }> = [
  { value: 'before_create', label: '创建前' },
  { value: 'after_create', label: '创建后' },
  { value: 'state_changed_to_pending', label: '状态变为待执行' },
  { value: 'state_changed_to_in_progress', label: '状态变为执行中' },
  { value: 'state_changed_to_completed', label: '状态变为已完成' },
  { value: 'state_changed_to_failed', label: '状态变为失败' },
  { value: 'before_delete', label: '删除前' },
  { value: 'after_delete', label: '删除后' },
];
