import { useState, useEffect } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty } from 'antd';
import { PlusOutlined, TagOutlined, ClockCircleOutlined, InboxOutlined } from '@ant-design/icons';
import { StatusPicker } from './StatusPicker';
import * as db from '../utils/database';
import { getExecutorOption } from '../types';
import { formatRelativeTime, formatLocalDateTime } from '../utils/datetime';

interface TodoListProps {
  onOpenCreateModal: () => void;
  onSelectTodo?: (todoId: string | number) => void;
  onOpenTagModal?: () => void;
}

function SkeletonRow() {
  return <div className="skeleton-row" />;
}

function SkeletonList() {
  return (
    <div style={{ padding: '12px 16px' }}>
      {Array.from({ length: 6 }).map((_, i) => (
        <SkeletonRow key={i} />
      ))}
    </div>
  );
}

export function TodoList({ onOpenCreateModal, onSelectTodo, onOpenTagModal }: TodoListProps) {
  const { state, dispatch } = useApp();
  const { todos, selectedTodoId, selectedTagId, tags } = state;
  const [isMobile, setIsMobile] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const checkMobile = () => setIsMobile(window.innerWidth < 768);
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => setIsLoading(false), 400);
    return () => clearTimeout(timer);
  }, []);

  const filteredTodos = selectedTagId
    ? todos.filter(t => (t as any).tag_ids?.includes(selectedTagId))
    : todos;

  if (isLoading) {
    return (
      <div className="todo-list-container">
        <div className="todo-list-header">
          <h3 style={{ margin: 0, fontWeight: 700, fontSize: 16, color: 'var(--color-text)' }}>我的任务</h3>
        </div>
        <SkeletonList />
      </div>
    );
  }

  return (
    <div className="todo-list-container">
      {/* Header */}
      <div className="todo-list-header">
        <h3 style={{ margin: 0, fontWeight: 700, fontSize: 16, color: 'var(--color-text)' }}>
          我的任务
          <span style={{ fontSize: 13, color: 'var(--color-text-tertiary)', fontWeight: 500, marginLeft: 8 }}>
            ({filteredTodos.length})
          </span>
        </h3>
        <div className="header-actions">
          <Button
            type="text"
            size="small"
            icon={<TagOutlined />}
            onClick={onOpenTagModal}
            className="tag-btn"
            aria-label="管理标签"
          />
          {!isMobile && (
            <Button
              type="primary"
              size="small"
              icon={<PlusOutlined />}
              className="action-btn action-btn-primary"
              onClick={onOpenCreateModal}
            >
              新建
            </Button>
          )}
        </div>
      </div>

      {/* Tag filter chips */}
      {tags.length > 0 && (
        <div className="tag-filter-bar">
          <button
            className={`tag-chip ${selectedTagId === null ? 'active' : ''}`}
            onClick={() => dispatch({ type: 'SELECT_TAG', payload: null })}
          >
            全部
          </button>
          {tags.map(tag => (
            <button
              key={tag.id}
              className={`tag-chip ${selectedTagId === tag.id ? 'active' : ''}`}
              style={{ '--tag-color': tag.color } as React.CSSProperties}
              onClick={() => dispatch({ type: 'SELECT_TAG', payload: tag.id })}
            >
              <span className="tag-dot" style={{ backgroundColor: tag.color }} />
              {tag.name}
            </button>
          ))}
        </div>
      )}

      {/* Todo list */}
      <div className="todo-list-content">
        {filteredTodos.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">
              <InboxOutlined />
            </div>
            <Empty
              description={
                <div style={{ color: 'var(--color-text-tertiary)', fontSize: 14 }}>
                  {selectedTagId ? '该标签下暂无任务' : '暂无任务'}
                  <br />
                  <span style={{ fontSize: 13, marginTop: 4, display: 'inline-block' }}>
                    点击右上角新建按钮创建第一个任务
                  </span>
                </div>
              }
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          </div>
        ) : (
          filteredTodos.map(todo => {
            const todoTags = tags.filter(t => (todo as any).tag_ids?.includes(t.id));
            const primaryTag = todoTags[0];
            const executor = todo.executor || 'claudecode';
            const executorOpt = getExecutorOption(executor);
            const isCompleted = todo.status === 'completed';

            return (
              <div
                key={todo.id}
                onClick={() => {
                  dispatch({ type: 'SELECT_TODO', payload: todo.id });
                  onSelectTodo?.(todo.id);
                }}
                className={`todo-item ${selectedTodoId === todo.id ? 'selected' : ''}`}
                style={{
                  cursor: 'pointer',
                  borderLeftColor: primaryTag?.color || '#cbd5e1',
                  borderLeftWidth: 4,
                  borderLeftStyle: 'solid',
                }}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    dispatch({ type: 'SELECT_TODO', payload: todo.id });
                    onSelectTodo?.(todo.id);
                  }
                }}
              >
                <div className="todo-item-content">
                  <div className="todo-item-main">
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
                      <div
                        className="todo-item-title"
                        style={{ opacity: isCompleted ? 0.6 : 1 }}
                      >
                        {todo.title}
                      </div>
                      <span
                        className="executor-badge"
                        style={{
                          backgroundColor: `${executorOpt.color}12`,
                          color: executorOpt.color,
                          border: `1px solid ${executorOpt.color}30`,
                        }}
                      >
                        {executorOpt.icon} {executorOpt.label}
                      </span>
                    </div>
                    {todo.prompt && (
                      <div className="todo-item-desc">
                        {todo.prompt.length > 60 ? todo.prompt.substring(0, 60) + '...' : todo.prompt}
                      </div>
                    )}
                    <div className="todo-item-tags" style={{ justifyContent: 'space-between' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexWrap: 'wrap' }}>
                        {todoTags.map(t => (
                          <span
                            key={t.id}
                            className="todo-tag-badge"
                            style={{
                              backgroundColor: t.color + '18',
                              color: t.color,
                              border: `1px solid ${t.color}30`,
                            }}
                          >
                            {t.name}
                          </span>
                        ))}
                        {todo.scheduler_config && (
                          <ClockCircleOutlined
                            style={{
                              fontSize: 12,
                              color: todo.scheduler_enabled ? 'var(--color-warning)' : 'var(--color-text-tertiary)',
                              marginLeft: todoTags.length > 0 ? 4 : 0,
                            }}
                          />
                        )}
                      </div>
                      <span
                        style={{
                          fontSize: 11,
                          color: 'var(--color-text-quaternary)',
                          flexShrink: 0,
                          marginLeft: 8,
                        }}
                        title={formatLocalDateTime(todo.updated_at)}
                      >
                        {formatRelativeTime(todo.updated_at)}
                      </span>
                    </div>
                  </div>
                  <div
                    className="todo-item-status"
                    aria-label="更改任务状态"
                  >
                    <StatusPicker
                      value={todo.status}
                      onChange={async (newStatus) => {
                        const updated = await db.updateTodo(
                          todo.id,
                          todo.title,
                          todo.prompt || '',
                          newStatus
                        );
                        dispatch({
                          type: 'UPDATE_TODO',
                          payload: updated
                        });
                      }}
                    />
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
