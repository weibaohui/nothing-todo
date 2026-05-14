import { useState, useMemo, useCallback } from 'react';
import { Input, App } from 'antd';
import { SearchOutlined } from '@ant-design/icons';
import { useApp } from '../hooks/useApp';
import { ExecutorBadge } from './ExecutorBadge';
import * as db from '../utils/database';
import { formatRelativeTime } from '../utils/datetime';
import type { Todo } from '../types';

/* ─── Column Definitions ─── */

interface ColumnDef {
  status: Todo['status'];
  label: string;
  color: string;
}

const COLUMNS: ColumnDef[] = [
  { status: 'pending',   label: '待办',     color: '#3b82f6' },
  { status: 'running',   label: '进行中',   color: '#f59e0b' },
  { status: 'completed', label: '已完成',   color: '#22c55e' },
  { status: 'failed',    label: '失败',     color: '#ef4444' },
];

/* ─── Helpers ─── */

function getColumnForStatus(status: Todo['status']): ColumnDef {
  return COLUMNS.find(c => c.status === status) || COLUMNS[0];
}

/* ─── Props ─── */

interface KanbanBoardProps {
  onSelectTodo?: (todoId: number) => void;
}

/* ─── Component ─── */

export function KanbanBoard({ onSelectTodo }: KanbanBoardProps) {
  const { state, dispatch } = useApp();
  const { message } = App.useApp();
  const { todos, tags, selectedTodoId } = state;

  const [searchText, setSearchText] = useState('');
  const [draggingId, setDraggingId] = useState<number | null>(null);
  const [dragOverStatus, setDragOverStatus] = useState<Todo['status'] | null>(null);

  /* ─── Filter by search ─── */
  const filteredTodos = useMemo(() => {
    if (!searchText.trim()) return todos;
    const q = searchText.toLowerCase();
    return todos.filter(t =>
      t.title.toLowerCase().includes(q) ||
      (t.prompt && t.prompt.toLowerCase().includes(q))
    );
  }, [todos, searchText]);

  /* ─── Group by status ─── */
  const grouped = useMemo(() => {
    const map: Record<Todo['status'], Todo[]> = {
      pending: [],
      running: [],
      completed: [],
      failed: [],
    };
    for (const todo of filteredTodos) {
      if (map[todo.status]) {
        map[todo.status].push(todo);
      } else {
        map.pending.push(todo);
      }
    }
    return map;
  }, [filteredTodos]);

  /* ─── Stats ─── */
  const totalCount = filteredTodos.length;
  const stats = useMemo(() => ({
    pending: grouped.pending.length,
    running: grouped.running.length,
    completed: grouped.completed.length,
    failed: grouped.failed.length,
  }), [grouped]);

  /* ─── Drag & Drop Handlers ─── */

  const handleDragStart = useCallback((todoId: number, e: React.DragEvent) => {
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', String(todoId));
    setDraggingId(todoId);
  }, []);

  const handleDragEnd = useCallback(() => {
    setDraggingId(null);
    setDragOverStatus(null);
  }, []);

  const handleDragOver = useCallback((status: Todo['status'], e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setDragOverStatus(status);
  }, []);

  const handleDragLeave = useCallback((status: Todo['status']) => {
    setDragOverStatus(prev => prev === status ? null : prev);
  }, []);

  const handleDrop = useCallback(async (targetStatus: Todo['status'], e: React.DragEvent) => {
    e.preventDefault();
    setDraggingId(null);
    setDragOverStatus(null);

    const todoId = parseInt(e.dataTransfer.getData('text/plain'), 10);
    if (isNaN(todoId)) return;

    const todo = todos.find(t => t.id === todoId);
    if (!todo || todo.status === targetStatus) return;

    try {
      const updated = await db.updateTodo(
        todoId,
        todo.title,
        todo.prompt || '',
        targetStatus,
        todo.executor,
      );
      dispatch({ type: 'UPDATE_TODO', payload: updated });
      message.success(`已移动到「${getColumnForStatus(targetStatus).label}」`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : '更新状态失败';
      message.error(msg);
    }
  }, [todos, dispatch, message]);

  /* ─── Click to select ─── */
  const handleCardClick = useCallback((todoId: number) => {
    dispatch({ type: 'SELECT_TODO', payload: todoId });
    onSelectTodo?.(todoId);
  }, [dispatch, onSelectTodo]);

  /* ─── Render Card ─── */
  const renderCard = (todo: Todo) => {
    const column = getColumnForStatus(todo.status);
    const todoTags = tags.filter(t => todo.tag_ids?.includes(t.id));
    const isDragging = draggingId === todo.id;

    return (
      <div
        key={todo.id}
        className={`kanban-card ${selectedTodoId === todo.id ? 'selected' : ''} ${isDragging ? 'dragging' : ''}`}
        draggable
        onDragStart={e => handleDragStart(todo.id, e)}
        onDragEnd={handleDragEnd}
        onClick={() => handleCardClick(todo.id)}
        role="button"
        tabIndex={0}
        onKeyDown={e => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            handleCardClick(todo.id);
          }
        }}
        style={{ borderLeftColor: column.color }}
      >
        {/* Title */}
        <div className="kanban-card-title" title={todo.title}>
          {todo.title}
        </div>

        {/* Prompt preview */}
        {todo.prompt && todo.prompt !== todo.title && (
          <div className="kanban-card-desc">
            {todo.prompt.length > 50 ? todo.prompt.slice(0, 50) + '…' : todo.prompt}
          </div>
        )}

        {/* Tags */}
        {todoTags.length > 0 && (
          <div className="kanban-card-tags">
            {todoTags.map(tag => (
              <span
                key={tag.id}
                className="kanban-tag-badge"
                style={{
                  backgroundColor: tag.color + '18',
                  color: tag.color,
                  border: `1px solid ${tag.color}30`,
                }}
              >
                {tag.name}
              </span>
            ))}
          </div>
        )}

        {/* Footer */}
        <div className="kanban-card-footer">
          {todo.executor && <ExecutorBadge executor={todo.executor} />}
          <span className="kanban-card-time">
            {formatRelativeTime(todo.updated_at)}
          </span>
        </div>
      </div>
    );
  };

  /* ─── Render Column ─── */
  const renderColumn = (column: ColumnDef) => {
    const items = grouped[column.status];
    const isOver = dragOverStatus === column.status;

    return (
      <div
        key={column.status}
        className={`kanban-column ${isOver ? 'drag-over' : ''}`}
        onDragOver={e => handleDragOver(column.status, e)}
        onDragLeave={() => handleDragLeave(column.status)}
        onDrop={e => handleDrop(column.status, e)}
      >
        {/* Column Header */}
        <div className="kanban-column-header" style={{ borderBottomColor: column.color }}>
          <div className="kanban-column-title">
            <div
              className="kanban-column-dot"
              style={{ backgroundColor: column.color }}
            />
            <span>{column.label}</span>
            <span className="kanban-column-count">{items.length}</span>
          </div>
        </div>

        {/* Column Body */}
        <div className="kanban-column-body">
          {items.length === 0 ? (
            <div className="kanban-column-empty">
              暂无任务
            </div>
          ) : (
            items.map(renderCard)
          )}
        </div>
      </div>
    );
  };

  /* ─── Render ─── */
  return (
    <div className="kanban-board">
      {/* Top Bar */}
      <div className="kanban-topbar">
        <div className="kanban-topbar-left">
          <Input
            className="kanban-search"
            placeholder="搜索任务…"
            prefix={<SearchOutlined style={{ color: 'var(--color-text-tertiary)' }} />}
            value={searchText}
            onChange={e => setSearchText(e.target.value)}
            allowClear
            size="small"
            style={{ width: 220 }}
          />
        </div>
        <div className="kanban-topbar-right">
          <span className="kanban-summary-item" style={{ color: '#3b82f6' }}>
            待办 <strong>{stats.pending}</strong>
          </span>
          <span className="kanban-summary-divider" />
          <span className="kanban-summary-item" style={{ color: '#f59e0b' }}>
            进行中 <strong>{stats.running}</strong>
          </span>
          <span className="kanban-summary-divider" />
          <span className="kanban-summary-item" style={{ color: '#22c55e' }}>
            已完成 <strong>{stats.completed}</strong>
          </span>
          <span className="kanban-summary-divider" />
          <span className="kanban-summary-item" style={{ color: '#ef4444' }}>
            失败 <strong>{stats.failed}</strong>
          </span>
          <span className="kanban-summary-divider" />
          <span className="kanban-summary-item" style={{ color: 'var(--color-text-secondary)' }}>
            共 <strong>{totalCount}</strong>
          </span>
        </div>
      </div>

      {/* Columns */}
      <div className="kanban-columns-container">
        {COLUMNS.map(renderColumn)}
      </div>
    </div>
  );
}
