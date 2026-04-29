import { useEffect, useRef } from 'react';
import { useApp } from './useApp';
import type { LogEntry, TodoItem } from '../types';

interface ExecEventStarted {
  type: 'Started';
  task_id: string;
  todo_id: number;
  todo_title: string;
  executor: string;
}

interface ExecEventOutput {
  type: 'Output';
  task_id: string;
  entry: LogEntry;
}

interface ExecEventFinished {
  type: 'Finished';
  task_id: string;
  todo_id: number;
  success: boolean;
  result: string | null;
}

interface ExecEventSync {
  type: 'Sync';
  tasks: Array<{
    task_id: string;
    todo_id: number;
    todo_title: string;
    executor: string;
    logs: string; // JSON string
  }>;
}

interface ExecEventTodoProgress {
  type: 'TodoProgress';
  task_id: string;
  progress: TodoItem[];
}

type ExecEvent = ExecEventStarted | ExecEventOutput | ExecEventFinished | ExecEventSync | ExecEventTodoProgress;

export function useExecutionEvents() {
  const { dispatch } = useApp();
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let shouldReconnect = true;

    function connect() {
      if (!shouldReconnect) return;

      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const ws = new WebSocket(`${protocol}//${window.location.host}/xyz/events`);
      wsRef.current = ws;

      ws.onopen = () => {
        console.log('[ExecutionEvents] WebSocket connected');
      };

      ws.onmessage = (event) => {
        // Legacy "Connected" message - ignore
        if (event.data === 'Connected') return;
        
        try {
          const data: ExecEvent = JSON.parse(event.data);

          switch (data.type) {
            case 'Sync': {
              // 后端发送当前实际运行的任务列表
              // 前端应清空当前 runningTasks 并用此列表初始化
              // 先移除所有当前任务
              dispatch({ type: 'CLEAR_RUNNING_TASKS' });
              
              // 添加后端发送的任务
              data.tasks.forEach(task => {
                let parsedLogs: LogEntry[] = [];
                try {
                  parsedLogs = JSON.parse(task.logs || '[]');
                } catch (e) {
                  console.error('[ExecutionEvents] Failed to parse logs:', e);
                }
                
                dispatch({
                  type: 'ADD_RUNNING_TASK',
                  payload: {
                    taskId: task.task_id,
                    todoId: task.todo_id,
                    todoTitle: task.todo_title,
                    executor: task.executor || 'claudecode',
                    logs: parsedLogs,
                    status: 'running',
                    startedAt: new Date().toISOString(),
                  },
                });
                
                // 更新对应的 todo 状态
                dispatch({
                  type: 'UPDATE_TODO_STATUS',
                  payload: { id: task.todo_id, status: 'running' },
                });
              });
              
              console.log(`[ExecutionEvents] Synced ${data.tasks.length} running tasks`);
              break;
            }
            case 'Started': {
              dispatch({
                type: 'ADD_RUNNING_TASK',
                payload: {
                  taskId: data.task_id,
                  todoId: data.todo_id,
                  todoTitle: data.todo_title,
                  executor: data.executor || 'claudecode',
                  logs: [],
                  status: 'running',
                  startedAt: new Date().toISOString(),
                },
              });
              dispatch({
                type: 'UPDATE_TODO_STATUS',
                payload: { id: data.todo_id, status: 'running' },
              });
              break;
            }
            case 'Output': {
              dispatch({
                type: 'APPEND_TASK_LOG',
                payload: { taskId: data.task_id, log: data.entry },
              });
              break;
            }
            case 'TodoProgress': {
              dispatch({
                type: 'UPDATE_TASK_TODO_PROGRESS',
                payload: { taskId: data.task_id, progress: data.progress },
              });
              break;
            }
            case 'Finished': {
              dispatch({
                type: 'FINISH_TASK',
                payload: {
                  taskId: data.task_id,
                  success: data.success,
                  result: data.result,
                },
              });
              // Sync todo status in real-time
              const newStatus = data.success ? 'completed' : 'failed';
              dispatch({
                type: 'UPDATE_TODO_STATUS',
                payload: { id: data.todo_id, status: newStatus },
              });
              // Auto-remove after 3 seconds
              setTimeout(() => {
                dispatch({ type: 'REMOVE_RUNNING_TASK', payload: data.task_id });
              }, 3000);
              break;
            }
          }
        } catch (e) {
          console.error('[ExecutionEvents] Failed to parse message:', e);
        }
      };

      ws.onclose = () => {
        console.log('[ExecutionEvents] WebSocket closed');
        wsRef.current = null;
        if (shouldReconnect) {
          reconnectTimerRef.current = setTimeout(() => {
            console.log('[ExecutionEvents] Reconnecting...');
            connect();
          }, 2000);
        }
      };

      ws.onerror = (error) => {
        console.error('[ExecutionEvents] WebSocket error:', error);
      };
    }

    connect();

    return () => {
      shouldReconnect = false;
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [dispatch]);
}
