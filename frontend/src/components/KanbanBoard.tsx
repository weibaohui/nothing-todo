import { useState, useMemo, useCallback } from 'react';
import { Input, App, Tag, Badge } from 'antd';
import {
  SearchOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  RobotOutlined,
  CopyOutlined,
} from '@ant-design/icons';
import XMarkdown from '@ant-design/x-markdown';
import { useApp } from '../hooks/useApp';
import { ExecutorBadge } from './ExecutorBadge';
import * as db from '../utils/database';
import { formatRelativeTime } from '../utils/datetime';
import type { Todo, ExecutionRecord } from '../types';

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

/* ─── Format helpers ─── */

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function formatDuration(ms: number): string {
  if (ms < 1_000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1_000).toFixed(0)}s`;
  return `${(ms / 60_000).toFixed(1)}m`;
}

/* ─── Component ─── */

export function KanbanBoard() {
  const { state, dispatch } = useApp();
  const { message } = App.useApp();
  const { todos, tags, selectedTodoId } = state;

  const [searchText, setSearchText] = useState('');
  const [draggingId, setDraggingId] = useState<number | null>(null);
  const [dragOverStatus, setDragOverStatus] = useState<Todo['status'] | null>(null);
  const [expandedPromptIds, setExpandedPromptIds] = useState<Set<number>>(new Set());
  const [expandedResultIds, setExpandedResultIds] = useState<Set<number>>(new Set());
  const [todoResults, setTodoResults] = useState<Record<number, string>>({});
  const [loadingResults, setLoadingResults] = useState<Set<number>>(new Set());

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

  /* ─── Toggle expand prompt ─── */
  const togglePrompt = useCallback((todoId: number) => {
    setExpandedPromptIds(prev => {
      const next = new Set(prev);
      if (next.has(todoId)) next.delete(todoId); else next.add(todoId);
      return next;
    });
  }, []);

  /* ─── Toggle expand result & lazy-fetch ─── */
  const toggleResult = useCallback(async (todo: Todo) => {
    const todoId = todo.id;

    // If not expanded yet and no cached result, try to fetch
    if (!expandedResultIds.has(todoId) && !todoResults[todoId]) {
      // Check state cache first
      const records = state.executionRecords[todoId];
      if (records && records.length > 0) {
        const latest = records[0];
        if (latest.result) {
          setTodoResults(prev => ({ ...prev, [todoId]: latest.result! }));
        }
      } else {
        // Lazy-fetch from API
        if (loadingResults.has(todoId)) return;
        setLoadingResults(prev => new Set(prev).add(todoId));
        try {
          const page = await db.getExecutionRecords(todoId, 1, 1);
          if (page.records.length > 0 && page.records[0].result) {
            setTodoResults(prev => ({ ...prev, [todoId]: page.records[0].result! }));
          }
        } catch {
          // silently ignore
        } finally {
          setLoadingResults(prev => { const n = new Set(prev); n.delete(todoId); return n; });
        }
      }
    }

    setExpandedResultIds(prev => {
      const next = new Set(prev);
      if (next.has(todoId)) next.delete(todoId); else next.add(todoId);
      return next;
    });
  }, [expandedResultIds, todoResults, loadingResults, state.executionRecords]);

  /* ─── Render Card ─── */
  const renderCard = (todo: Todo) => {
    const column = getColumnForStatus(todo.status);
    const todoTags = tags.filter(t => todo.tag_ids?.includes(t.id));
    const isDragging = draggingId === todo.id;
    const isSuccess = todo.status === 'completed';
    const isFinished = todo.status === 'completed' || todo.status === 'failed';
    const promptExpanded = expandedPromptIds.has(todo.id);
    const resultExpanded = expandedResultIds.has(todo.id);
    const resultText = todoResults[todo.id] || '';
    const isLoadingResult = loadingResults.has(todo.id);
    const records = state.executionRecords[todo.id];
    const todoExecutionRecord: ExecutionRecord | undefined = records?.length > 0 ? records[0] : undefined;

    return (
      <div
        key={todo.id}
        className={`kanban-card ${selectedTodoId === todo.id ? 'selected' : ''} ${isDragging ? 'dragging' : ''} ${isFinished && resultText ? 'has-result' : ''}`}
        draggable
        onDragStart={e => handleDragStart(todo.id, e)}
        onDragEnd={handleDragEnd}
        style={{ borderTop: `3px solid ${column.color}` }}
      >
        {/* Card Header — Title + Status Icon */}
        <div className="kanban-card-header">
          <div className="kanban-card-top">
            <span className="kanban-card-title" title={todo.title}>
              {todo.title}
            </span>
            {isFinished && (
              isSuccess ? (
                <CheckCircleOutlined className="kanban-status-icon kanban-status-success" />
              ) : (
                <CloseCircleOutlined className="kanban-status-icon kanban-status-failed" />
              )
            )}
          </div>

          {/* Meta Row */}
          <div className="kanban-card-meta-row">
            {todo.executor && <ExecutorBadge executor={todo.executor} />}
            <span className="kanban-card-meta-time">
              <ClockCircleOutlined /> {formatRelativeTime(todo.updated_at)}
            </span>
            {todoExecutionRecord?.model && (
              <span className="kanban-card-meta-model">
                <RobotOutlined /> {todoExecutionRecord.model}
              </span>
            )}
          </div>

          {/* Tags */}
          {todoTags.length > 0 && (
            <div className="kanban-card-tags">
              {todoTags.map(tag => (
                <Tag key={tag.id} color={tag.color} className="kanban-tag-badge">
                  {tag.name}
                </Tag>
              ))}
            </div>
          )}
        </div>

        {/* Card Body — Expandable Sections */}
        <div className="kanban-card-body">
          {/* Prompt Section */}
          {todo.prompt && todo.prompt !== todo.title && (
            <div className="kanban-card-section">
              <div
                className="kanban-card-section-header kanban-section-prompt"
                onClick={e => { e.stopPropagation(); togglePrompt(todo.id); }}
                role="button"
                tabIndex={0}
                onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); togglePrompt(todo.id); } }}
              >
                <span className="kanban-card-section-label">📋 Prompt</span>
                {todo.prompt && (
                  <button
                    className="kanban-copy-btn"
                    onClick={e => {
                      e.stopPropagation();
                      navigator.clipboard.writeText(todo.prompt).then(() => message.success('已复制'));
                    }}
                    title="复制 Prompt"
                  >
                    <CopyOutlined />
                  </button>
                )}
                <span className="kanban-card-section-toggle">
                  {promptExpanded ? '收起' : '展开'}
                </span>
              </div>
              {promptExpanded && (
                <div className="kanban-card-section-content">
                  <XMarkdown content={todo.prompt} />
                </div>
              )}
            </div>
          )}

          {/* Result Section (completed/failed only) */}
          {isFinished && (
            <div className="kanban-card-section">
              <div
                className={`kanban-card-section-header kanban-section-result ${isSuccess ? 'result-success' : 'result-failed'}`}
                onClick={e => { e.stopPropagation(); toggleResult(todo); }}
                role="button"
                tabIndex={0}
                onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); toggleResult(todo); } }}
              >
                <span className="kanban-card-section-label">✅ 结论</span>
                {resultText && (
                  <button
                    className="kanban-copy-btn"
                    onClick={e => {
                      e.stopPropagation();
                      navigator.clipboard.writeText(resultText).then(() => message.success('已复制'));
                    }}
                    title="复制结论"
                  >
                    <CopyOutlined />
                  </button>
                )}
                <span className="kanban-card-section-toggle">
                  {isLoadingResult ? '加载中…' : (resultExpanded ? '收起' : '展开')}
                </span>
              </div>
              {resultExpanded && (
                <div className="kanban-card-section-content">
                  {isLoadingResult ? (
                    <span className="kanban-loading-text">加载中…</span>
                  ) : resultText ? (
                    <XMarkdown content={resultText} />
                  ) : (
                    <span className="kanban-no-result">暂无结论</span>
                  )}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer — Usage stats */}
        <div className="kanban-card-footer">
          {todoExecutionRecord?.usage && (
            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'center', fontSize: 11, color: 'var(--color-text-tertiary)' }}>
              {todoExecutionRecord.usage.duration_ms != null && (
                <span>{formatDuration(todoExecutionRecord.usage.duration_ms)}</span>
              )}
              <span>
                {formatTokens(todoExecutionRecord.usage.input_tokens)} + {formatTokens(todoExecutionRecord.usage.output_tokens)} tokens
              </span>
              {todoExecutionRecord.usage.total_cost_usd != null && todoExecutionRecord.usage.total_cost_usd > 0 && (
                <span>${todoExecutionRecord.usage.total_cost_usd.toFixed(4)}</span>
              )}
              {todoExecutionRecord.trigger_type && todoExecutionRecord.trigger_type !== 'manual' && (
                <Badge
                  count={todoExecutionRecord.trigger_type === 'scheduler' ? '定时' : todoExecutionRecord.trigger_type}
                  style={{ fontSize: 10, height: 16, lineHeight: '16px' }}
                />
              )}
            </div>
          )}
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
