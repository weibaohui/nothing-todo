import { useState, useEffect } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty } from 'antd';
import { PlusOutlined } from '@ant-design/icons';

interface TodoListProps {
  onOpenCreateModal: () => void;
  onSelectTodo?: (todoId: string | number) => void;
}

const statusColors: Record<string, string> = {
  pending: '#999',
  running: '#1890ff',
  completed: '#52c41a',
  failed: '#ff4d4f',
};

const statusLabels: Record<string, string> = {
  pending: '待执行',
  running: '执行中',
  completed: '已完成',
  failed: '失败',
};

export function TodoList({ onOpenCreateModal, onSelectTodo }: TodoListProps) {
  const { state, dispatch } = useApp();
  const { todos, selectedTodoId, selectedTagId } = state;
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const checkMobile = () => setIsMobile(window.innerWidth < 768);
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  const filteredTodos = selectedTagId ? todos : todos;

  return (
    <div className="todo-list-container">
      {!isMobile && (
        <div className="todo-list-header">
          <h3 style={{ margin: 0, fontWeight: 600, fontSize: 16 }}>我的任务</h3>
          <Button
            type="primary"
            size="small"
            icon={<PlusOutlined />}
            className="action-btn action-btn-primary"
            onClick={onOpenCreateModal}
          >
            新建
          </Button>
        </div>
      )}

      <div className="todo-list-content">
        {filteredTodos.length === 0 ? (
          <div className="empty-state">
            <Empty description="暂无任务" image={Empty.PRESENTED_IMAGE_SIMPLE} />
          </div>
        ) : (
          filteredTodos.map(todo => (
            <div
              key={todo.id}
              onClick={() => {
                dispatch({ type: 'SELECT_TODO', payload: todo.id });
                onSelectTodo?.(todo.id);
              }}
              className={`todo-item ${selectedTodoId === todo.id ? 'selected' : ''}`}
              style={{ cursor: 'pointer' }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 12, width: '100%' }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className={`todo-item-title ${todo.status === 'completed' ? 'completed' : ''}`}>
                    {todo.title}
                  </div>
                  {todo.description && (
                    <div className="todo-item-desc">
                      {todo.description.substring(0, 60)}{todo.description.length > 60 ? '...' : ''}
                    </div>
                  )}
                </div>
                <span
                  className="todo-status-badge"
                  style={{
                    backgroundColor: statusColors[todo.status],
                    color: '#fff',
                    flexShrink: 0,
                  }}
                >
                  {statusLabels[todo.status]}
                </span>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
