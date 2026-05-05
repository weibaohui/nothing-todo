import axios from 'axios';
import type { Todo, Tag, ExecutionRecord, ExecutionSummary, ExecutionRecordsPage, ExecutorSkills, SkillComparison, PaginatedInvocations } from '../types';

interface ApiResp<T> {
  code: number;
  data: T | null;
  message: string;
}

const api = axios.create({
  baseURL: '',
  headers: { 'Content-Type': 'application/json' },
});

api.interceptors.response.use(
  (res) => {
    const body = res.data as ApiResp<unknown>;
    if (body && body.code !== 0) {
      return Promise.reject(new Error(body.message || `Error ${body.code}`));
    }
    return res;
  },
  (error) => {
    if (error.response?.data?.message) {
      return Promise.reject(new Error(error.response.data.message));
    }
    return Promise.reject(error);
  },
);

function unwrap<T>(res: { data: ApiResp<T> }): T {
  return res.data.data as T;
}

// Todo APIs

export async function getAllTodos(): Promise<Todo[]> {
  return unwrap(await api.get<ApiResp<Todo[]>>('/xyz/todos'));
}

export async function createTodo(title: string, prompt: string = '', tagIds: number[] = []): Promise<Todo> {
  return unwrap(await api.post<ApiResp<Todo>>('/xyz/todos', { title, prompt, tag_ids: tagIds }));
}

export async function updateTodo(
  id: number,
  title: string,
  prompt: string,
  status: string,
  executor?: string,
  scheduler_enabled?: boolean,
  scheduler_config?: string | null,
  workspace?: string | null,
): Promise<Todo> {
  const body: Record<string, unknown> = { title, prompt, status };
  if (executor !== undefined) body.executor = executor;
  if (scheduler_enabled !== undefined) body.scheduler_enabled = scheduler_enabled;
  if (scheduler_config !== undefined) body.scheduler_config = scheduler_config;
  if (workspace !== undefined) body.workspace = workspace;

  return unwrap(await api.put<ApiResp<Todo>>(`/xyz/todos/${id}`, body));
}

export async function deleteTodo(id: number): Promise<void> {
  await api.delete(`/xyz/todos/${id}`);
}

export async function forceUpdateTodoStatus(id: number, status: string): Promise<Todo> {
  return unwrap(await api.put<ApiResp<Todo>>(`/xyz/todos/${id}/force-status`, { status }));
}

export async function updateTodoTags(todoId: number, tagIds: number[]): Promise<void> {
  await api.put(`/xyz/todos/${todoId}/tags`, { tag_ids: tagIds });
}

// Tag APIs

export async function getAllTags(): Promise<Tag[]> {
  return unwrap(await api.get<ApiResp<Tag[]>>('/xyz/tags'));
}

export async function createTag(name: string, color: string): Promise<Tag> {
  return unwrap(await api.post<ApiResp<Tag>>('/xyz/tags', { name, color }));
}

export async function deleteTag(id: number): Promise<void> {
  await api.delete(`/xyz/tags/${id}`);
}

// Execution APIs

export async function getExecutionRecords(todoId: number, page?: number, limit?: number): Promise<ExecutionRecordsPage> {
  const params: Record<string, unknown> = { todo_id: todoId };
  if (page !== undefined) params.page = page;
  if (limit !== undefined) params.limit = limit;
  return unwrap(await api.get<ApiResp<ExecutionRecordsPage>>(`/xyz/execution-records`, { params }));
}

export async function getExecutionRecord(recordId: number): Promise<ExecutionRecord> {
  return unwrap(await api.get<ApiResp<ExecutionRecord>>(`/xyz/execution-records/${recordId}`));
}

export async function getExecutionRecordsBySession(sessionId: string): Promise<ExecutionRecord[]> {
  return unwrap(await api.get<ApiResp<ExecutionRecord[]>>(`/xyz/execution-records/session/${encodeURIComponent(sessionId)}`));
}

export async function executeTodo(todoId: number, message: string, executor?: string): Promise<{ task_id: string }> {
  return unwrap(await api.post<ApiResp<{ task_id: string }>>('/xyz/execute', { todo_id: todoId, message, executor }));
}

export async function getExecutionSummary(todoId: number): Promise<ExecutionSummary> {
  return unwrap(await api.get<ApiResp<ExecutionSummary>>(`/xyz/todos/${todoId}/summary`));
}

export async function getDashboardStats(): Promise<import('../types').DashboardStats> {
  return unwrap(await api.get<ApiResp<import('../types').DashboardStats>>('/xyz/dashboard-stats'));
}

export async function stopExecution(recordId: number): Promise<void> {
  await api.post('/xyz/execute/stop', { record_id: recordId });
}

export async function resumeExecutionRecord(recordId: number, message?: string): Promise<{ task_id: string; record_id: number }> {
  return unwrap(await api.post<ApiResp<{ task_id: string; record_id: number }>>(`/xyz/execution-records/${recordId}/resume`, { message }));
}

// Scheduler APIs

export async function updateScheduler(
  id: number,
  scheduler_enabled: boolean,
  scheduler_config: string | null,
): Promise<Todo> {
  return unwrap(await api.put<ApiResp<Todo>>(`/xyz/todos/${id}/scheduler`, { scheduler_enabled, scheduler_config }));
}

export async function getSchedulerTodos(): Promise<Todo[]> {
  return unwrap(await api.get<ApiResp<Todo[]>>('/xyz/scheduler/todos'));
}

export async function getRunningTodos(): Promise<Todo[]> {
  return unwrap(await api.get<ApiResp<Todo[]>>('/xyz/running-todos'));
}

// Backup APIs

export async function exportBackup(): Promise<string> {
  const res = await api.get('/xyz/backup/export', {
    headers: { 'Accept': 'application/x-yaml' },
    responseType: 'text',
    transformResponse: [(data) => data],
  });
  if (typeof res.data === 'string') return res.data;
  return JSON.stringify(res.data);
}

export async function importBackup(yamlContent: string): Promise<string> {
  return unwrap(await api.post<ApiResp<string>>('/xyz/backup/import', yamlContent, {
    headers: { 'Content-Type': 'application/x-yaml' },
  }));
}

export async function mergeBackup(tags: { name: string; color: string }[], todos: { title: string; prompt: string; status: string; executor?: string; scheduler_enabled: boolean; scheduler_config?: string; tag_names: string[]; workspace?: string }[]): Promise<string> {
  return unwrap(await api.post<ApiResp<string>>('/xyz/backup/merge', { tags, todos }));
}

export async function exportSelectedBackup(todoIds: number[]): Promise<string> {
  const res = await api.post('/xyz/backup/export-selected', { todo_ids: todoIds }, {
    headers: { 'Accept': 'application/x-yaml' },
    responseType: 'text',
    transformResponse: [(data: unknown) => data],
  });
  if (typeof res.data === 'string') return res.data;
  return JSON.stringify(res.data);
}

export async function triggerLocalBackup(): Promise<string> {
  return unwrap(await api.post<ApiResp<string>>('/xyz/backup/database/trigger'));
}

export async function getDatabaseBackupStatus(): Promise<{
  auto_backup_enabled: boolean;
  auto_backup_cron: string;
  last_backup: string | null;
  files: { name: string; size: number; created_at: string }[];
}> {
  return unwrap(await api.get<ApiResp<{
    auto_backup_enabled: boolean;
    auto_backup_cron: string;
    last_backup: string | null;
    files: { name: string; size: number; created_at: string }[];
  }>>('/xyz/backup/database/status'));
}

export async function updateAutoBackup(enabled: boolean, cron: string): Promise<string> {
  return unwrap(await api.put<ApiResp<string>>('/xyz/backup/database/auto', { enabled, cron }));
}

export async function deleteBackupFile(filename: string): Promise<string> {
  return unwrap(await api.delete<ApiResp<string>>('/xyz/backup/database/file', { data: { filename } }));
}

// Config APIs

export async function getConfig(): Promise<import('../types').Config> {
  return unwrap(await api.get<ApiResp<import('../types').Config>>('/xyz/config'));
}

export async function updateConfig(config: import('../types').Config): Promise<import('../types').Config> {
  return unwrap(await api.put<ApiResp<import('../types').Config>>('/xyz/config', config));
}

// Skills APIs

export async function getSkillsList(): Promise<ExecutorSkills[]> {
  return unwrap(await api.get<ApiResp<ExecutorSkills[]>>('/xyz/skills'));
}

export async function getSkillsComparison(): Promise<SkillComparison[]> {
  return unwrap(await api.get<ApiResp<SkillComparison[]>>('/xyz/skills/compare'));
}

export async function syncSkill(sourceExecutor: string, skillName: string, targetExecutors: string[]): Promise<string> {
  return unwrap(await api.post<ApiResp<string>>('/xyz/skills/sync', {
    source_executor: sourceExecutor,
    skill_name: skillName,
    target_executors: targetExecutors,
  }));
}

export async function getSkillInvocations(params?: { page?: number; limit?: number; skill_name?: string; executor?: string }): Promise<PaginatedInvocations> {
  return unwrap(await api.get<ApiResp<PaginatedInvocations>>('/xyz/skills/invocations', { params }));
}

export async function recordSkillInvocation(data: { skill_name: string; executor: string; todo_id: number; status: string; duration_ms?: number }): Promise<number> {
  return unwrap(await api.post<ApiResp<number>>('/xyz/skills/invocations', data));
}

// Skill content & files
export interface SkillFileInfo {
  path: string;
  size: number;
  modified_at: string;
}

export interface SkillContent {
  skill_name: string;
  executor: string;
  content: string;
  files: SkillFileInfo[];
}

export async function getSkillContent(executor: string, skillName: string): Promise<SkillContent> {
  return unwrap(await api.get<ApiResp<SkillContent>>('/xyz/skills/content', {
    params: { executor, skill_name: skillName },
  }));
}

// Export skill as .tar.gz (returns blob)
export async function exportSkill(executor: string, skillName: string): Promise<Blob> {
  const response = await api.get('/xyz/skills/export', {
    params: { executor, skill_name: skillName },
    responseType: 'blob',
  });
  return response.data;
}

// Import skill from file
export interface ImportResult {
  skill_name: string;
  imported_files: number;
  message: string;
}

export async function importSkill(executor: string, file: File, skillName?: string, flatten?: boolean): Promise<ImportResult> {
  const formData = new FormData();
  formData.append('file', file);
  if (skillName) formData.append('skill_name', skillName);
  if (flatten !== undefined) formData.append('flatten', String(flatten));

  const response = await api.post<ApiResp<ImportResult>>('/xyz/skills/import', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
    params: { executor, skill_name: skillName, flatten },
  });
  return response.data.data as ImportResult;
}
