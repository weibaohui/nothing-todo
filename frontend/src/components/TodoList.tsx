import { useState, useEffect } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty } from 'antd';
import { PlusOutlined, TagOutlined } from '@ant-design/icons';

interface TodoListProps {
  onOpenCreateModal: () => void;
  onSelectTodo?: (todoId: string | number) => void;
  onOpenTagModal?: () => void;
}

const statusColors: Record<string, string> = {
  pending: '#d9d9d9',
  running: '#1890ff',
  completed: '#52c41a',
  failed: '#ff4d4f',
};

export function TodoList({ onOpenCreateModal, onSelectTodo, onOpenTagModal }: TodoListProps) {
  const { state, dispatch } = useApp();
  const { todos, selectedTodoId, selectedTagId, tags } = state;
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const checkMobile = () => setIsMobile(window.innerWidth < 768);
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  const filteredTodos = selectedTagId
    ? todos.filter(t => (t as any).tag_ids?.includes(selectedTagId))
    : todos;

  return (
    <div className="todo-list-container">
      {/* Header with tag filters */}
      <div className="todo-list-header">
        <h3 style={{ margin: 0, fontWeight: 600, fontSize: 16 }}>我的任务</h3>
        <div className="header-actions">
          <Button
            type="text"
            size="small"
            icon={<TagOutlined />}
            onClick={onOpenTagModal}
            className="tag-btn"
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
              style={{
                '--tag-color': tag.color,
              } as React.CSSProperties}
              onClick={() => dispatch({ type: 'SELECT_TAG', payload: tag.id })}
            >
              <span className="tag-dot" style={{ backgroundColor: tag.color }} />
              {tag.name}
            </button>
          ))}
        </div>
      )}

      <div className="todo-list-content">
        {filteredTodos.length === 0 ? (
          <div className="empty-state">
            <Empty description="暂无任务" image={Empty.PRESENTED_IMAGE_SIMPLE} />
          </div>
        ) : (
          filteredTodos.map(todo => {
            const todoTags = tags.filter(t => (todo as any).tag_ids?.includes(t.id));
            const primaryTag = todoTags[0];
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
                  borderLeftColor: primaryTag?.color || 'transparent',
                  borderLeftWidth: primaryTag ? 4 : 0,
                  borderLeftStyle: 'solid',
                }}
              >
                <div className="todo-item-content">
                  <div className="todo-item-main">
                    <div className={`todo-item-title ${todo.status === 'completed' ? 'completed' : ''}`}>
                      {todo.title}
                    </div>
                    {todo.description && (
                      <div className="todo-item-desc">
                        {todo.description.substring(0, 60)}{todo.description.length > 60 ? '...' : ''}
                      </div>
                    )}
                    {todoTags.length > 0 && (
                      <div className="todo-item-tags">
                        {todoTags.map(t => (
                          <span key={t.id} className="todo-tag-badge" style={{ backgroundColor: t.color + '20', color: t.color }}>
                            {t.name}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                  <div className="todo-item-status">
                    <span
                      className="status-circle"
                      style={{ backgroundColor: statusColors[todo.status] }}
                      title={todo.status}
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
