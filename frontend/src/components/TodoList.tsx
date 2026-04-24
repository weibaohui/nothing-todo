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
  failed: '执行失败',
};

export function TodoList({ onOpenCreateModal, onSelectTodo }: TodoListProps) {
  const { state, dispatch } = useApp();
  const { todos, selectedTodoId, selectedTagId } = state;

  // TODO: filter by actual tag relations (need to query todo_tags table)
  const filteredTodos = selectedTagId ? todos : todos;

  return (
    <div style={{ width: 350, borderRight: '1px solid #f0f0f0', padding: 16, height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <h3 style={{ margin: 0 }}>Todo 列表</h3>
        <Button type="primary" size="small" icon={<PlusOutlined />} onClick={onOpenCreateModal}>
          新建
        </Button>
      </div>

      <div style={{ flex: 1, overflow: 'auto' }}>
        {filteredTodos.length === 0 ? (
          <Empty description="暂无 Todo" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        ) : (
          filteredTodos.map(todo => (
            <div
              key={todo.id}
              onClick={() => {
                dispatch({ type: 'SELECT_TODO', payload: todo.id });
                onSelectTodo?.(todo.id);
              }}
              style={{
                padding: 12,
                marginBottom: 8,
                borderRadius: 8,
                border: '1px solid #f0f0f0',
                cursor: 'pointer',
                background: selectedTodoId === todo.id ? '#f5f5f5' : '#fff',
                transition: 'all 0.2s',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <div style={{ flex: 1 }}>
                  <div style={{
                    fontWeight: 500,
                    marginBottom: 4,
                    textDecoration: todo.status === 'completed' ? 'line-through' : 'none',
                    color: todo.status === 'completed' ? '#999' : 'inherit',
                  }}>
                    {todo.title}
                  </div>
                  {todo.description && (
                    <div style={{ fontSize: 12, color: '#666', marginBottom: 8 }}>
                      {todo.description.substring(0, 50)}{todo.description.length > 50 ? '...' : ''}
                    </div>
                  )}
                </div>
                <div style={{
                  fontSize: 12,
                  padding: '2px 8px',
                  borderRadius: 10,
                  backgroundColor: statusColors[todo.status],
                  color: '#fff',
                }}>
                  {statusLabels[todo.status]}
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
