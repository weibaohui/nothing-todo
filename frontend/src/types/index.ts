export interface Todo {
  id: number;
  title: string;
  prompt: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  tag_ids: number[];
  executor?: string;
  scheduler_enabled?: boolean;
  scheduler_config?: string | null;
  scheduler_next_run_at?: string | null;
  task_id?: string | null;
}

export interface Tag {
  id: number;
  name: string;
  color: string;
  created_at: string;
}

export interface TodoTag {
  todo_id: number;
  tag_id: number;
}

export interface LogEntry {
  timestamp: string;
  type: 'info' | 'stdout' | 'stderr' | 'error' | 'text' | 'tool' | 'step_start' | 'step_finish' | 'result' | 'assistant' | 'user' | 'system' | 'thinking';
  content: string;
}

export interface ExecutionRecord {
  id: number;
  todo_id: number;
  status: 'running' | 'success' | 'failed';
  command: string;
  stdout: string;
  stderr: string;
  logs: string;
  result: string | null;
  started_at: string;
  finished_at: string | null;
  usage: ExecutionUsage | null;
  executor: string | null;
  model: string | null;
  trigger_type: string;
}

export interface ExecutionUsage {
  input_tokens: number;
  output_tokens: number;
  cache_read_input_tokens: number | null;
  cache_creation_input_tokens: number | null;
  total_cost_usd: number | null;
  duration_ms: number | null;
}

export interface ExecutionSummary {
  todo_id: number;
  total_executions: number;
  success_count: number;
  failed_count: number;
  running_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read_tokens: number;
  total_cache_creation_tokens: number;
  total_cost_usd: number | null;
}

export interface ExecutionRecordsPage {
  records: ExecutionRecord[];
  total: number;
  page: number;
  limit: number;
}

export interface ExecuteResult {
  success: boolean;
  stdout: string;
  stderr: string;
  logs: LogEntry[];
}

export interface RunningTask {
  taskId: string;
  todoId: number;
  todoTitle: string;
  executor: string;
  logs: LogEntry[];
  status: 'running' | 'finished';
  success?: boolean;
  result?: string | null;
  startedAt: string;
  finishedAt?: string;
}

export interface ExecutorOption {
  value: string;
  label: string;
  color: string;
  icon: string;
}

export const EXECUTORS: ExecutorOption[] = [
  { value: 'claudecode', label: 'Claude',     color: '#7c3aed', icon: '🟣' },
  { value: 'codebuddy',  label: 'CodeBuddy',  color: '#2563eb', icon: '🔵' },
  { value: 'opencode',   label: 'Opencode',   color: '#f59e0b', icon: '🟡' },
  { value: 'joinai',     label: 'JoinAI',     color: '#0d9488', icon: '🟢' },
];

export function getExecutorOption(value: string): ExecutorOption {
  return EXECUTORS.find(e => e.value === value) || EXECUTORS[0];
}
