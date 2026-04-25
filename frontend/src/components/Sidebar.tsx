import { useApp } from '../hooks/useApp';
import { Button, Popconfirm } from 'antd';
import { PlusOutlined, DeleteOutlined, AppstoreOutlined } from '@ant-design/icons';

interface SidebarProps {
  onOpenTagModal: () => void;
}

export function Sidebar({ onOpenTagModal }: SidebarProps) {
  const { state, dispatch } = useApp();
  const { tags, selectedTagId, todos } = state;

  return (
    <div className="sidebar-container" style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div className="sidebar-header">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h3 className="sidebar-title">标签</h3>
          <Button
            type="text"
            size="small"
            icon={<PlusOutlined />}
            onClick={onOpenTagModal}
            className="icon-btn"
            aria-label="新建标签"
          />
        </div>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '8px 0' }}>
        <div
          key="all"
          onClick={() => dispatch({ type: 'SELECT_TAG', payload: null })}
          className={`sidebar-item ${selectedTagId === null ? 'active' : ''}`}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              dispatch({ type: 'SELECT_TAG', payload: null });
            }
          }}
        >
          <AppstoreOutlined style={{ fontSize: 14 }} />
          <span style={{ flex: 1 }}>全部 Todo</span>
          <span style={{ fontSize: 12, color: 'var(--color-text-tertiary)', fontWeight: 500 }}>
            {todos.length}
          </span>
        </div>

        {tags.map(tag => (
          <div
            key={tag.id}
            onClick={() => dispatch({ type: 'SELECT_TAG', payload: tag.id })}
            className={`sidebar-item ${selectedTagId === tag.id ? 'active' : ''}`}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                dispatch({ type: 'SELECT_TAG', payload: tag.id });
              }
            }}
          >
            <span
              style={{
                width: 8,
                height: 8,
                borderRadius: '50%',
                backgroundColor: tag.color,
                flexShrink: 0,
              }}
            />
            <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {tag.name}
            </span>
            <Popconfirm
              title="删除标签"
              description="确定要删除这个标签吗？"
              onConfirm={(e) => {
                e?.stopPropagation();
              }}
              onCancel={(e) => e?.stopPropagation()}
            >
              <DeleteOutlined
                onClick={(e) => e.stopPropagation()}
                style={{
                  fontSize: 12,
                  color: 'var(--color-text-tertiary)',
                  padding: 4,
                  borderRadius: 4,
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  (e.target as HTMLElement).style.color = 'var(--color-error)';
                  (e.target as HTMLElement).style.background = 'var(--color-error-bg)';
                }}
                onMouseLeave={(e) => {
                  (e.target as HTMLElement).style.color = 'var(--color-text-tertiary)';
                  (e.target as HTMLElement).style.background = 'transparent';
                }}
              />
            </Popconfirm>
          </div>
        ))}
      </div>
    </div>
  );
}
