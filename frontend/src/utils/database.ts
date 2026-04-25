import axios from 'axios';
import type { Todo, Tag, ExecutionRecord, ExecutionSummary } from '../types';

const api = axios.create({
  baseURL: '',
  headers: { 'Content-Type': 'application/json' },
  transformResponse: [(data) => {
    if (typeof data === 'string') {
      try { return JSON.parse(data); } catch { return data; }
    }
    return data;
  }],
});

// Todo APIs

export async function getAllTodos(): Promise<Todo[]> {
  const res = await api.get<Todo[]>('/xyz/todos');
  return res.data;
}

export async function createTodo(title: string, description: string = '', tagIds: number[] = []): Promise<Todo> {
  const res = await api.post<Todo>('/xyz/todos', { title, description, tag_ids: tagIds });
  return res.data;
}

export async function updateTodo(
  id: number,
  title: string,
  description: string,
  status: string,
  executor?: string,
  scheduler_enabled?: boolean,
  scheduler_config?: string | null,
): Promise<Todo> {
  const body: Record<string, unknown> = { title, description, status };
  if (executor !== undefined) body.executor = executor;
  if (scheduler_enabled !== undefined) body.scheduler_enabled = scheduler_enabled;
  if (scheduler_config !== undefined) body.scheduler_config = scheduler_config;

  const res = await api.put<Todo>(`/xyz/todos/${id}`, body);
  return res.data;
}

export async function deleteTodo(id: number): Promise<void> {
  await api.delete(`/xyz/todos/${id}`);
}

export async function forceUpdateTodoStatus(id: number, status: string): Promise<Todo> {
  const res = await api.put<Todo>(`/xyz/todos/${id}/force-status`, { status });
  return res.data;
}

export async function updateTodoTags(todoId: number, tagIds: number[]): Promise<void> {
  await api.put(`/xyz/todos/${todoId}/tags`, { tag_ids: tagIds });
}

// Tag APIs

export async function getAllTags(): Promise<Tag[]> {
  const res = await api.get<Tag[]>('/xyz/tags');
  return res.data;
}

export async function createTag(name: string, color: string): Promise<Tag> {
  const res = await api.post<Tag>('/xyz/tags', { name, color });
  return res.data;
}

export async function deleteTag(id: number): Promise<void> {
  await api.delete(`/xyz/tags/${id}`);
}

// Execution APIs

export async function getExecutionRecords(todoId: number): Promise<ExecutionRecord[]> {
  const res = await api.get<ExecutionRecord[]>(`/xyz/execution-records`, { params: { todo_id: todoId } });
  return res.data;
}

export async function executeTodo(todoId: number, message: string, executor?: string): Promise<{ task_id: string }> {
  const res = await api.post<{ task_id: string }>('/xyz/execute', { todo_id: todoId, message, executor });
  return res.data;
}

export async function getExecutionSummary(todoId: number): Promise<ExecutionSummary> {
  const res = await api.get<ExecutionSummary>(`/xyz/todos/${todoId}/summary`);
  return res.data;
}

// Scheduler APIs

export async function updateScheduler(
  id: number,
  scheduler_enabled: boolean,
  scheduler_config: string | null,
): Promise<Todo> {
  const res = await api.put<Todo>(`/xyz/todos/${id}/scheduler`, { scheduler_enabled, scheduler_config });
  return res.data;
}

export async function getSchedulerTodos(): Promise<Todo[]> {
  const res = await api.get<Todo[]>('/xyz/scheduler/todos');
  return res.data;
}
