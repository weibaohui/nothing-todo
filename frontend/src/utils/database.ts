import axios from 'axios';
import type { Todo, Tag, ExecutionRecord, ExecutionSummary, ExecutionRecordsPage } from '../types';

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

// Config APIs

export async function getConfig(): Promise<import('../types').Config> {
  return unwrap(await api.get<ApiResp<import('../types').Config>>('/xyz/config'));
}

export async function updateConfig(config: import('../types').Config): Promise<import('../types').Config> {
  return unwrap(await api.put<ApiResp<import('../types').Config>>('/xyz/config', config));
}
