import React, { createContext, useContext, useReducer, useEffect, ReactNode } from 'react';
import { Todo, Tag, ExecutionRecord, RunningTask, LogEntry, TodoItem, ExecutionStats } from '../types';
import * as db from '../utils/database';

interface AppState {
  todos: Todo[];
  tags: Tag[];
  selectedTodoId: number | null;
  selectedTagId: number | null;
  executionRecords: Record<number, ExecutionRecord[]>;
  loading: boolean;
  runningTasks: Record<string, RunningTask>;
  activeTaskId: string | null;
}

type Action =
  | { type: 'SET_TODOS'; payload: Todo[] }
  | { type: 'SET_TAGS'; payload: Tag[] }
  | { type: 'ADD_TODO'; payload: Todo }
  | { type: 'UPDATE_TODO'; payload: Todo }
  | { type: 'DELETE_TODO'; payload: number }
  | { type: 'SELECT_TODO'; payload: number | null }
  | { type: 'SELECT_TAG'; payload: number | null }
  | { type: 'ADD_TAG'; payload: Tag }
  | { type: 'DELETE_TAG'; payload: number }
  | { type: 'SET_EXECUTION_RECORDS'; payload: { todoId: number; records: ExecutionRecord[] } }
  | { type: 'ADD_EXECUTION_RECORD'; payload: { todoId: number; record: ExecutionRecord } }
  | { type: 'UPDATE_EXECUTION_RECORD'; payload: { todoId: number; record: ExecutionRecord } }
  | { type: 'UPDATE_TODO_STATUS'; payload: { id: number; status: string } }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'ADD_RUNNING_TASK'; payload: RunningTask }
  | { type: 'APPEND_TASK_LOG'; payload: { taskId: string; log: LogEntry } }
  | { type: 'FINISH_TASK'; payload: { taskId: string; success: boolean; result: string | null } }
  | { type: 'REMOVE_RUNNING_TASK'; payload: string }
  | { type: 'CLEAR_RUNNING_TASKS' }
  | { type: 'SET_ACTIVE_TASK'; payload: string | null }
  | { type: 'UPDATE_TASK_TODO_PROGRESS'; payload: { taskId: string; progress: TodoItem[] } }
  | { type: 'UPDATE_TASK_EXECUTION_STATS'; payload: { taskId: string; stats: ExecutionStats } };

const initialState: AppState = {
  todos: [],
  tags: [],
  selectedTodoId: null,
  selectedTagId: null,
  executionRecords: {},
  loading: true,
  runningTasks: {},
  activeTaskId: null,
};

function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case 'SET_TODOS':
      return { ...state, todos: action.payload };
    case 'SET_TAGS':
      return { ...state, tags: action.payload };
    case 'ADD_TODO':
      return { ...state, todos: [action.payload, ...state.todos] };
    case 'UPDATE_TODO':
      return {
        ...state,
        todos: state.todos.map(t => t.id === action.payload.id ? action.payload : t),
      };
    case 'DELETE_TODO':
      return { ...state, todos: state.todos.filter(t => t.id !== action.payload) };
    case 'SELECT_TODO':
      return { ...state, selectedTodoId: action.payload };
    case 'SELECT_TAG':
      return { ...state, selectedTagId: action.payload };
    case 'ADD_TAG':
      return { ...state, tags: [...state.tags, action.payload] };
    case 'DELETE_TAG':
      return { ...state, tags: state.tags.filter(t => t.id !== action.payload) };
    case 'SET_EXECUTION_RECORDS':
      return {
        ...state,
        executionRecords: {
          ...state.executionRecords,
          [action.payload.todoId]: action.payload.records,
        },
      };
    case 'ADD_EXECUTION_RECORD':
      return {
        ...state,
        executionRecords: {
          ...state.executionRecords,
          [action.payload.todoId]: [
            action.payload.record,
            ...(state.executionRecords[action.payload.todoId] || []),
          ],
        },
      };
    case 'UPDATE_EXECUTION_RECORD':
      return {
        ...state,
        executionRecords: {
          ...state.executionRecords,
          [action.payload.todoId]: (state.executionRecords[action.payload.todoId] || []).map(
            r => r.id === action.payload.record.id ? action.payload.record : r
          ),
        },
      };
    case 'UPDATE_TODO_STATUS':
      return {
        ...state,
        todos: state.todos.map(t =>
          t.id === action.payload.id
            ? { ...t, status: action.payload.status as Todo['status'], updated_at: new Date().toISOString() }
            : t
        ),
      };
    case 'SET_LOADING':
      return { ...state, loading: action.payload };
    case 'ADD_RUNNING_TASK': {
      const task = action.payload;
      return {
        ...state,
        runningTasks: { ...state.runningTasks, [task.taskId]: task },
        activeTaskId: state.activeTaskId || task.taskId,
      };
    }
    case 'APPEND_TASK_LOG': {
      const { taskId, log } = action.payload;
      const task = state.runningTasks[taskId];
      if (!task) return state;
      return {
        ...state,
        runningTasks: {
          ...state.runningTasks,
          [taskId]: { ...task, logs: [...task.logs, log] },
        },
      };
    }
    case 'FINISH_TASK': {
      const { taskId, success, result } = action.payload;
      const task = state.runningTasks[taskId];
      if (!task) return state;
      const now = new Date().toISOString();
      return {
        ...state,
        runningTasks: {
          ...state.runningTasks,
          [taskId]: {
            ...task,
            status: 'finished' as const,
            success,
            result,
            finishedAt: now,
          },
        },
      };
    }
    case 'REMOVE_RUNNING_TASK': {
      const taskId = action.payload;
      const { [taskId]: _, ...rest } = state.runningTasks;
      const remainingIds = Object.keys(rest);
      return {
        ...state,
        runningTasks: rest,
        activeTaskId: state.activeTaskId === taskId
          ? (remainingIds[0] || null)
          : state.activeTaskId,
      };
    }
    case 'CLEAR_RUNNING_TASKS': {
      return {
        ...state,
        runningTasks: {},
        activeTaskId: null,
      };
    }
    case 'SET_ACTIVE_TASK':
      return { ...state, activeTaskId: action.payload };
    case 'UPDATE_TASK_TODO_PROGRESS': {
      const task = state.runningTasks[action.payload.taskId];
      if (!task) return state;
      return {
        ...state,
        runningTasks: {
          ...state.runningTasks,
          [action.payload.taskId]: { ...task, todoProgress: action.payload.progress },
        },
      };
    }
    case 'UPDATE_TASK_EXECUTION_STATS': {
      const task = state.runningTasks[action.payload.taskId];
      if (!task) return state;
      return {
        ...state,
        runningTasks: {
          ...state.runningTasks,
          [action.payload.taskId]: { ...task, executionStats: action.payload.stats },
        },
      };
    }
    default:
      return state;
  }
}

const AppContext = createContext<{
  state: AppState;
  dispatch: React.Dispatch<Action>;
} | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    async function loadData() {
      try {
        const todos = await db.getAllTodos();
        const tags = await db.getAllTags();
        dispatch({ type: 'SET_TODOS', payload: todos });
        dispatch({ type: 'SET_TAGS', payload: tags });

        // 注意：running tasks 现在由 WebSocket 的 Sync 事件初始化
        // 不再从数据库加载 running todos，因为数据库状态可能与实际进程状态不同步
      } catch {
        // silently ignore load failures
      } finally {
        dispatch({ type: 'SET_LOADING', payload: false });
      }
    }
    loadData();
  }, []);

  return (
    <AppContext.Provider value={{ state, dispatch }}>
      {children}
    </AppContext.Provider>
  );
}

export function useApp() {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useApp must be used within AppProvider');
  }
  
  const clearSelection = () => {
    context.dispatch({ type: 'SELECT_TODO', payload: null });
    context.dispatch({ type: 'SELECT_TAG', payload: null });
  };
  
  return { ...context, clearSelection };
}
