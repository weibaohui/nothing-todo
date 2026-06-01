import { api, unwrap } from './client';

export interface HookFilter {
  title_contains: string | null;
  tags: number[];
}

export interface HookAction {
  target_todo_id: number;
  prompt_template?: string | null;
  skip_if_missing?: boolean;
}

export interface HookRule {
  id: number;
  name: string;
  description: string | null;
  enabled: boolean;
  trigger: string;
  filter: HookFilter;
  action: HookAction;
  is_async: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface GlobalHookConfig {
  enabled: boolean;
  default_timeout_secs: number;
  max_concurrency: number;
}

export interface HookLogEntry {
  id: number;
  hook_id: number | null;
  hook_name: string | null;
  trigger: string;
  todo_id: number | null;
  args_sent: string | null;
  env_sent: string | null;
  exit_code: number | null;
  stdout: string | null;
  stderr: string | null;
  duration_ms: number | null;
  success: boolean | null;
  error_msg: string | null;
  created_at: string;
}

export interface HookLogPage {
  logs: HookLogEntry[];
  total: number;
  page: number;
  limit: number;
}

export interface TodoHookConfig {
  todo_id: number;
  hook_mode: string;
  override_enabled: boolean;
  rule_ids: number[];
}

export async function getHooks(): Promise<HookRule[]> {
  return unwrap(await api.get('/xyz/hooks'));
}

export async function getHook(id: number): Promise<HookRule> {
  return unwrap(await api.get(`/xyz/hooks/${id}`));
}

export interface CreateHookRequest {
  name: string;
  description?: string;
  enabled: boolean;
  trigger: string;
  filter?: HookFilter;
  action: HookAction;
  is_async: boolean;
}

export async function createHook(req: CreateHookRequest): Promise<HookRule> {
  return unwrap(await api.post('/xyz/hooks', req));
}

export interface UpdateHookRequest {
  name?: string;
  description?: string;
  enabled?: boolean;
  trigger?: string;
  filter?: HookFilter;
  action?: HookAction;
  is_async?: boolean;
}

export async function updateHook(id: number, req: UpdateHookRequest): Promise<HookRule> {
  return unwrap(await api.put(`/xyz/hooks/${id}`, req));
}

export async function deleteHook(id: number): Promise<void> {
  await api.delete(`/xyz/hooks/${id}`);
}

export interface TestHookResult {
  success: boolean;
  exit_code: number | null;
  stdout: string;
  stderr: string;
  duration_ms: number;
  error_msg: string | null;
}

export async function testHook(id: number): Promise<TestHookResult> {
  return unwrap(await api.post(`/xyz/hooks/${id}/test`));
}

export async function getGlobalHookConfig(): Promise<GlobalHookConfig> {
  return unwrap(await api.get('/xyz/hooks/config'));
}

export async function updateGlobalHookConfig(req: Partial<GlobalHookConfig>): Promise<GlobalHookConfig> {
  return unwrap(await api.put('/xyz/hooks/config', req));
}

export async function setGlobalDefaultHooks(hookIds: number[]): Promise<void> {
  await api.post('/xyz/hooks/defaults', hookIds);
}

export async function getGlobalDefaultHooks(): Promise<number[]> {
  return unwrap(await api.get('/xyz/hooks/defaults'));
}

export async function getHookLogs(params?: {
  hook_id?: number;
  todo_id?: number;
  status?: string;
  page?: number;
  limit?: number;
}): Promise<HookLogPage> {
  return unwrap(await api.get('/xyz/hooks/logs', { params }));
}

export async function clearHookLogs(): Promise<number> {
  return unwrap(await api.delete('/xyz/hooks/logs'));
}

export async function getTodoHookConfig(todoId: number): Promise<TodoHookConfig> {
  return unwrap(await api.get(`/xyz/todos/${todoId}/hooks`));
}

export async function updateTodoHookConfig(
  todoId: number,
  req: { hook_mode?: string; override_enabled?: boolean; rule_ids?: number[] }
): Promise<TodoHookConfig> {
  return unwrap(await api.put(`/xyz/todos/${todoId}/hooks`, req));
}

export const HOOK_TRIGGERS = [
  { value: 'before_create', label: 'Before Create (创建前)' },
  { value: 'after_create', label: 'After Create (创建后)' },
  { value: 'state_changed_to_pending', label: '状态变为待执行' },
  { value: 'state_changed_to_in_progress', label: '状态变为执行中' },
  { value: 'state_changed_to_completed', label: '状态变为已完成' },
  { value: 'state_changed_to_failed', label: '状态变为失败' },
  { value: 'before_delete', label: 'Before Delete (删除前)' },
  { value: 'after_delete', label: 'After Delete (删除后)' },
] as const;

export const HOOK_MODES = [
  { value: 'inherit', label: 'Inherit (继承全局)' },
  { value: 'custom', label: 'Custom (自定义)' },
  { value: 'disabled', label: 'Disabled (禁用)' },
] as const;
