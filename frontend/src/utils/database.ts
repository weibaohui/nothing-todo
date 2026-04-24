const API_BASE = '';

export async function getAllTodos(): Promise<any[]> {
  const res = await fetch(`${API_BASE}/api/todos`);
  return res.json();
}

export async function createTodo(title: string, description: string = ''): Promise<number> {
  const res = await fetch(`${API_BASE}/api/todos`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, description }),
  });
  const data = await res.json();
  return data.id;
}

export async function updateTodo(id: number, title: string, description: string, status: string): Promise<void> {
  await fetch(`${API_BASE}/api/todos/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, description, status }),
  });
}

export async function deleteTodo(id: number): Promise<void> {
  await fetch(`${API_BASE}/api/todos/${id}`, {
    method: 'DELETE',
  });
}

export async function getAllTags(): Promise<any[]> {
  const res = await fetch(`${API_BASE}/api/tags`);
  return res.json();
}

export async function createTag(name: string, color: string): Promise<number> {
  const res = await fetch(`${API_BASE}/api/tags`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, color }),
  });
  const data = await res.json();
  return data.id;
}

export async function deleteTag(id: number): Promise<void> {
  await fetch(`${API_BASE}/api/tags/${id}`, {
    method: 'DELETE',
  });
}

export async function getExecutionRecords(todoId: number): Promise<any[]> {
  const res = await fetch(`${API_BASE}/api/execution-records?todo_id=${todoId}`);
  return res.json();
}

export interface ExecuteResponse {
  task_id: string;
}

export async function executeJoinai(todoId: number, message: string, executor?: string): Promise<ExecuteResponse> {
  const res = await fetch(`${API_BASE}/api/execute`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ todo_id: todoId, message, executor }),
  });
  return res.json();
}

export async function forceUpdateTodoStatus(id: number, status: string): Promise<void> {
  await fetch(`${API_BASE}/api/todos/${id}/force-status`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ status }),
  });
}

export async function getExecutionSummary(todoId: number): Promise<any> {
  const res = await fetch(`${API_BASE}/api/todos/${todoId}/summary`);
  return res.json();
}
