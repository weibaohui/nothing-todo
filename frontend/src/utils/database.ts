import type { Todo, Tag, ExecutionRecord, ExecutionSummary } from '../types';

const API_BASE = '';

class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

async function request<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${url}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new ApiError(res.status, text || `HTTP ${res.status}`);
  }

  if (res.status === 204 || res.headers.get('content-length') === '0') {
    return undefined as T;
  }

  return res.json();
}

// Todo APIs

export async function getAllTodos(): Promise<Todo[]> {
  return request<Todo[]>('/xyz/todos');
}

export async function createTodo(title: string, description: string = '', tagIds: number[] = []): Promise<Todo> {
  return request<Todo>('/xyz/todos', {
    method: 'POST',
    body: JSON.stringify({ title, description, tag_ids: tagIds }),
  });
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

  return request<Todo>(`/xyz/todos/${id}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

export async function deleteTodo(id: number): Promise<void> {
  return request<void>(`/xyz/todos/${id}`, { method: 'DELETE' });
}

export async function forceUpdateTodoStatus(id: number, status: string): Promise<Todo> {
  return request<Todo>(`/xyz/todos/${id}/force-status`, {
    method: 'PUT',
    body: JSON.stringify({ status }),
  });
}

export async function updateTodoTags(todoId: number, tagIds: number[]): Promise<void> {
  return request<void>(`/xyz/todos/${todoId}/tags`, {
    method: 'PUT',
    body: JSON.stringify({ tag_ids: tagIds }),
  });
}

// Tag APIs

export async function getAllTags(): Promise<Tag[]> {
  return request<Tag[]>('/xyz/tags');
}

export async function createTag(name: string, color: string): Promise<Tag> {
  return request<Tag>('/xyz/tags', {
    method: 'POST',
    body: JSON.stringify({ name, color }),
  });
}

export async function deleteTag(id: number): Promise<void> {
  return request<void>(`/xyz/tags/${id}`, { method: 'DELETE' });
}

// Execution APIs

export async function getExecutionRecords(todoId: number): Promise<ExecutionRecord[]> {
  return request<ExecutionRecord[]>(`/xyz/execution-records?todo_id=${todoId}`);
}

export async function executeTodo(todoId: number, message: string, executor?: string): Promise<{ task_id: string }> {
  return request<{ task_id: string }>('/xyz/execute', {
    method: 'POST',
    body: JSON.stringify({ todo_id: todoId, message, executor }),
  });
}

export async function getExecutionSummary(todoId: number): Promise<ExecutionSummary> {
  return request<ExecutionSummary>(`/xyz/todos/${todoId}/summary`);
}

// Scheduler APIs

export async function updateScheduler(
  id: number,
  scheduler_enabled: boolean,
  scheduler_config: string | null,
): Promise<Todo> {
  return request<Todo>(`/xyz/todos/${id}/scheduler`, {
    method: 'PUT',
    body: JSON.stringify({ scheduler_enabled, scheduler_config }),
  });
}

export async function getSchedulerTodos(): Promise<Todo[]> {
  return request<Todo[]>('/xyz/scheduler/todos');
}
