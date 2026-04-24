import React, { createContext, useContext, useReducer, useEffect, ReactNode } from 'react';
import { Todo, Tag, ExecutionRecord } from '../types';
import * as db from '../utils/database';

interface AppState {
  todos: Todo[];
  tags: Tag[];
  selectedTodoId: number | null;
  selectedTagId: number | null;
  executionRecords: Record<number, ExecutionRecord[]>;
  loading: boolean;
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
  | { type: 'SET_LOADING'; payload: boolean };

const initialState: AppState = {
  todos: [],
  tags: [],
  selectedTodoId: null,
  selectedTagId: null,
  executionRecords: {},
  loading: true,
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
    case 'SET_LOADING':
      return { ...state, loading: action.payload };
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
      } catch (error) {
        console.error('Failed to load data:', error);
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
  return context;
}