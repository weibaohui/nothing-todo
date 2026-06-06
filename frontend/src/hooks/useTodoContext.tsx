import React, { createContext, useContext, useReducer, useMemo, ReactNode } from 'react';
import type { Todo, Tag } from '../types';

// ─── State & Reducer ─────────────────────────────────────────

interface TodoState {
  todos: Todo[];
  tags: Tag[];
  selectedTodoId: number | null;
  selectedTagId: number | null;
}

type TodoAction =
  | { type: 'SET_TODOS'; payload: Todo[] }
  | { type: 'SET_TAGS'; payload: Tag[] }
  | { type: 'ADD_TODO'; payload: Todo }
  | { type: 'UPDATE_TODO'; payload: Todo }
  | { type: 'DELETE_TODO'; payload: number }
  | { type: 'SELECT_TODO'; payload: number | null }
  | { type: 'SELECT_TAG'; payload: number | null }
  | { type: 'ADD_TAG'; payload: Tag }
  | { type: 'DELETE_TAG'; payload: number }
  | { type: 'UPDATE_TODO_STATUS'; payload: { id: number; status: string } };

const initialState: TodoState = {
  todos: [],
  tags: [],
  selectedTodoId: null,
  selectedTagId: null,
};

function reducer(state: TodoState, action: TodoAction): TodoState {
  switch (action.type) {
    case 'SET_TODOS': return { ...state, todos: action.payload };
    case 'SET_TAGS': return { ...state, tags: action.payload };
    case 'ADD_TODO': return { ...state, todos: [action.payload, ...state.todos] };
    case 'UPDATE_TODO': return { ...state, todos: state.todos.map(t => t.id === action.payload.id ? action.payload : t) };
    case 'DELETE_TODO': return { ...state, todos: state.todos.filter(t => t.id !== action.payload) };
    case 'SELECT_TODO': return { ...state, selectedTodoId: action.payload };
    case 'SELECT_TAG': return { ...state, selectedTagId: action.payload };
    case 'ADD_TAG': return { ...state, tags: [...state.tags, action.payload] };
    case 'DELETE_TAG': return { ...state, tags: state.tags.filter(t => t.id !== action.payload) };
    case 'UPDATE_TODO_STATUS':
      return {
        ...state,
        todos: state.todos.map(t =>
          t.id === action.payload.id
            ? { ...t, status: action.payload.status as Todo['status'], updated_at: new Date().toISOString() }
            : t
        ),
      };
    default: return state;
  }
}

// ─── Context ──────────────────────────────────────────────────

const TodoContext = createContext<{ state: TodoState; dispatch: React.Dispatch<TodoAction> } | null>(null);

// ─── Provider (used inside AppProvider) ───────────────────────

export function TodoProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);
  const ctx = useMemo(() => ({ state, dispatch }), [state, dispatch]);
  return <TodoContext.Provider value={ctx}>{children}</TodoContext.Provider>;
}

// ─── Hook ─────────────────────────────────────────────────────

export function useTodos() {
  const ctx = useContext(TodoContext);
  if (!ctx) throw new Error('useTodos must be used within TodoProvider');
  return ctx;
}

export type { TodoAction };
