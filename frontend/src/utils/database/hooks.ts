/** Inline hook items attached to a todo. Hooks live in the `todos.hooks`
 *  column as a JSON array — there is no global hook library or rules engine.
 *
 *  The only triggers are per-target-state: each fires when the source todo
 *  transitions INTO that status. The `{{message}}` placeholder inside the
 *  target todo's prompt is filled with the source todo's most recent
 *  successful execution `result`. If the source has not run yet, the
 *  source's `prompt` is used as a fallback.
 *
 *  This matches the manual "execute with args" flow — the user writes the
 *  template by editing the target todo's prompt, hooks just supply the
 *  `{{message}}` value automatically. */

export type HookTrigger =
  | 'state_changed_to_pending'
  | 'state_changed_to_in_progress'
  | 'state_changed_to_completed'
  | 'state_changed_to_failed';

/**
 * Policy for how the rating gate treats an unrated record. Matches the
 * backend `UnratedPolicy` enum (snake_case serialization).
 */
export type UnratedPolicy = 'skip' | 'pass';

export interface TodoHookItem {
  id: number;
  trigger: HookTrigger;
  target_todo_id: number;
  skip_if_missing?: boolean;
  enabled: boolean;
  /**
   * Optional rating gate. When set (0-100), the hook only fires if the
   * source todo's most recent FINISHED execution record has
   * `rating >= min_rating`. `undefined`/`null` means no gate (always fire).
   */
  min_rating?: number | null;
  /**
   * What to do when the source todo has no rating on its latest finished
   * record. Defaults to `'skip'` (do not fire). `'pass'` treats the missing
   * rating as if it had passed the gate.
   */
  unrated_policy?: UnratedPolicy;
}

export const HOOK_TRIGGERS: ReadonlyArray<{ value: HookTrigger; label: string }> = [
  { value: 'state_changed_to_pending', label: '状态变为待执行' },
  { value: 'state_changed_to_in_progress', label: '状态变为执行中' },
  { value: 'state_changed_to_completed', label: '状态变为已完成' },
  { value: 'state_changed_to_failed', label: '状态变为失败' },
];

export const UNRATED_POLICIES: ReadonlyArray<{ value: UnratedPolicy; label: string; description: string }> = [
  {
    value: 'skip',
    label: '未评分时跳过',
    description: '安全默认：未评分视为不达标，不触发下游',
  },
  {
    value: 'pass',
    label: '未评分时通过',
    description: '宽松：只有明确低分才会拦截，未评分时正常触发',
  },
];

/** Default rating gate when the user opens the modal fresh. */
export const DEFAULT_MIN_RATING: number | null = null;
export const DEFAULT_UNRATED_POLICY: UnratedPolicy = 'skip';

const HOOK_TRIGGER_LABEL_BY_VALUE: Record<HookTrigger, string> = HOOK_TRIGGERS.reduce(
  (acc, t) => ({ ...acc, [t.value]: t.label }),
  {} as Record<HookTrigger, string>,
);

/** Look up the Chinese label for a `hook:<trigger>` trigger_type. */
export function getHookTriggerLabel(triggerType: string): string | null {
  if (!triggerType.startsWith('hook:')) return null;
  const key = triggerType.slice('hook:'.length) as HookTrigger;
  return HOOK_TRIGGER_LABEL_BY_VALUE[key] ?? triggerType;
}
