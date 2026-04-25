const API_BASE = '';

export async function getAllTodos(): Promise<any[]> {
  const res = await fetch(`${API_BASE}/xyz/todos`);
  return res.json();
}

export async function createTodo(title: string, description: string = '', tagIds: number[] = []): Promise<number> {
  const res = await fetch(`${API_BASE}/xyz/todos`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, description, tag_ids: tagIds }),
  });
  const data = await res.json();
  return data.id;
}

export async function updateTodo(
  id: number,
  title: string,
  description: string,
  status: string,
  executor?: string,
  scheduler_enabled?: boolean,
  scheduler_config?: string | null,
): Promise<void> {
  const body: any = { title, description, status };
  if (executor !== undefined) body.executor = executor;
  if (scheduler_enabled !== undefined) body.scheduler_enabled = scheduler_enabled;
  if (scheduler_config !== undefined) body.scheduler_config = scheduler_config;

  await fetch(`${API_BASE}/xyz/todos/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

export async function deleteTodo(id: number): Promise<void> {
  await fetch(`${API_BASE}/xyz/todos/${id}`, {
    method: 'DELETE',
  });
}

export async function getAllTags(): Promise<any[]> {
  const res = await fetch(`${API_BASE}/xyz/tags`);
  return res.json();
}

export async function createTag(name: string, color: string): Promise<number> {
  const res = await fetch(`${API_BASE}/xyz/tags`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, color }),
  });
  const data = await res.json();
  return data.id;
}

export async function deleteTag(id: number): Promise<void> {
  await fetch(`${API_BASE}/xyz/tags/${id}`, {
    method: 'DELETE',
  });
}

export async function getExecutionRecords(todoId: number): Promise<any[]> {
  const res = await fetch(`${API_BASE}/xyz/execution-records?todo_id=${todoId}`);
  return res.json();
}

export interface ExecuteResponse {
  task_id: string;
}

export async function executeJoinai(todoId: number, message: string, executor?: string): Promise<ExecuteResponse> {
  const res = await fetch(`${API_BASE}/xyz/execute`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ todo_id: todoId, message, executor }),
  });
  return res.json();
}

export async function forceUpdateTodoStatus(id: number, status: string): Promise<void> {
  await fetch(`${API_BASE}/xyz/todos/${id}/force-status`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ status }),
  });
}

export async function getExecutionSummary(todoId: number): Promise<any> {
  const res = await fetch(`${API_BASE}/xyz/todos/${todoId}/summary`);
  return res.json();
}

export async function updateTodoTags(todoId: number, tagIds: number[]): Promise<void> {
  await fetch(`${API_BASE}/xyz/todos/${todoId}/tags`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ tag_ids: tagIds }),
  });
}

export async function addTodoTag(todoId: number, tagId: number): Promise<void> {
  await fetch(`${API_BASE}/xyz/todos/${todoId}/tags/${tagId}`, {
    method: 'POST',
  });
}

export async function removeTodoTag(todoId: number, tagId: number): Promise<void> {
  await fetch(`${API_BASE}/xyz/todos/${todoId}/tags/${tagId}`, {
    method: 'DELETE',
  });
}

// Scheduler APIs
export async function getSchedulerTodos(): Promise<any[]> {
  const res = await fetch(`${API_BASE}/xyz/scheduler/todos`);
  return res.json();
}
