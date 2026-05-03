import type { ReactNode } from 'react';
import { FaSquare } from 'react-icons/fa';

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
  workspace?: string | null;
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
  type: 'info' | 'stdout' | 'stderr' | 'error' | 'text' | 'tool' | 'tool_use' | 'tool_call' | 'tool_result' | 'step_start' | 'step_finish' | 'result' | 'assistant' | 'user' | 'system' | 'thinking' | 'tokens';
  content: string;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system' | 'tool' | 'thinking' | 'result';
  content: string;
  timestamp?: string;
  toolName?: string;
  toolInput?: string;
  toolResult?: string;
  isCollapsed?: boolean;
}

export interface TodoItem {
  id?: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
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
  pid: number | null;
  task_id?: string | null;
  session_id?: string | null;
  todo_progress?: string | null;
  execution_stats?: ExecutionStats | null;
}

export interface ExecutionUsage {
  input_tokens: number;
  output_tokens: number;
  cache_read_input_tokens: number | null;
  cache_creation_input_tokens: number | null;
  total_cost_usd: number | null;
  duration_ms: number | null;
}

export interface ExecutionStats {
  tool_calls: number;
  conversation_turns: number;
  thinking_count: number;
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

export interface ExecutorCount {
  executor: string;
  count: number;
  execution_count: number;
  success_count: number;
  failed_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost_usd: number;
}

export interface TagCount {
  tag_id: number;
  tag_name: string;
  tag_color: string;
  count: number;
  execution_count: number;
  success_count: number;
  failed_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost_usd: number;
}

export interface ModelCount {
  model: string;
  count: number;
  execution_count: number;
  success_count: number;
  failed_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read_tokens: number;
  total_cache_creation_tokens: number;
  total_cost_usd: number;
}

export interface DailyExecution {
  date: string;
  success: number;
  failed: number;
}

export interface DailyTokenStats {
  date: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_tokens: number;
  total_cost_usd: number;
}

export interface DashboardStats {
  total_todos: number;
  pending_todos: number;
  running_todos: number;
  completed_todos: number;
  failed_todos: number;
  total_tags: number;
  scheduled_todos: number;
  total_executions: number;
  success_executions: number;
  failed_executions: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read_tokens: number;
  total_cache_creation_tokens: number;
  total_cost_usd: number;
  avg_duration_ms: number;
  executor_distribution: ExecutorCount[];
  tag_distribution: TagCount[];
  model_distribution: ModelCount[];
  daily_executions: DailyExecution[];
  daily_token_stats: DailyTokenStats[];
  recent_executions: ExecutionRecord[];
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
  todoProgress?: TodoItem[];
  executionStats?: ExecutionStats;
}

export interface ExecutorOption {
  value: string;
  label: string;
  color: string;
  icon: ReactNode;
}

export const EXECUTORS: ExecutorOption[] = [
  { value: 'claudecode', label: 'Claude',    color: '#e17055', icon: <FaSquare color="#e17055" size={14} /> },
  { value: 'codebuddy',  label: 'CodeBuddy', color: '#00b894', icon: <FaSquare color="#00b894" size={14} /> },
  { value: 'opencode',   label: 'Opencode',  color: '#fdcb6e', icon: <FaSquare color="#fdcb6e" size={14} /> },
  { value: 'joinai',     label: 'JoinAI',    color: '#6c5ce7', icon: <FaSquare color="#6c5ce7" size={14} /> },
  { value: 'atomcode',   label: 'AtomCode',  color: '#e84393', icon: <FaSquare color="#e84393" size={14} /> },
  { value: 'hermes',     label: 'Hermes',    color: '#0984e3', icon: <FaSquare color="#0984e3" size={14} /> },
  { value: 'kimi',       label: 'Kimi',      color: '#d63031', icon: <FaSquare color="#d63031" size={14} /> },
  { value: 'codex',      label: 'Codex',     color: '#488597', icon: <FaSquare color="#488597" size={14} /> },
];

export interface ExecutorPaths {
  opencode: string;
  hermes: string;
  joinai: string;
  claude_code: string;
  codebuddy: string;
  kimi: string;
  atomcode: string;
  codex: string;
}

export interface Config {
  port: number;
  host: string;
  db_path: string;
  log_level: string;
  executors: ExecutorPaths;
}

export const RESUMABLE_EXECUTORS = new Set(['claudecode', 'kimi', 'opencode', 'joinai']);

export function supportsResume(record: ExecutionRecord): boolean {
  return (
    record.status !== 'running' &&
    !!record.session_id &&
    !!record.executor &&
    RESUMABLE_EXECUTORS.has(record.executor.toLowerCase())
  );
}

export function getExecutorOption(value: string): ExecutorOption {
  return EXECUTORS.find(e => e.value === value.toLowerCase()) || EXECUTORS[0];
}
