import { useApp } from '../hooks/useApp';
import { Button, Popconfirm } from 'antd';
import { PlusOutlined, DeleteOutlined } from '@ant-design/icons';

interface SidebarProps {
  onOpenTagModal: () => void;
}

export function Sidebar({ onOpenTagModal }: SidebarProps) {
  const { state, dispatch } = useApp();
  const { tags, selectedTagId, todos } = state;

  return (
    <div style={{ width: 220, borderRight: '1px solid #f0f0f0', padding: 16, height: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <h3 style={{ margin: 0 }}>标签</h3>
        <Button type="text" size="small" icon={<PlusOutlined />} onClick={onOpenTagModal} />
      </div>

      <div
        key="all"
        onClick={() => dispatch({ type: 'SELECT_TAG', payload: null })}
        style={{
          padding: '8px 12px',
          marginBottom: 4,
          cursor: 'pointer',
          borderRadius: 6,
          background: selectedTagId === null ? '#e6f7ff' : 'transparent',
          color: selectedTagId === null ? '#1890ff' : 'inherit',
          fontWeight: selectedTagId === null ? 500 : 400,
        }}
      >
        全部 Todo ({todos.length})
      </div>

      {tags.map(tag => (
        <div
          key={tag.id}
          onClick={() => dispatch({ type: 'SELECT_TAG', payload: tag.id })}
          style={{
            padding: '8px 12px',
            marginBottom: 4,
            cursor: 'pointer',
            borderRadius: 6,
            background: selectedTagId === tag.id ? '#e6f7ff' : 'transparent',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              backgroundColor: tag.color,
            }} />
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
              style={{ fontSize: 12, color: '#999' }}
            />
          </Popconfirm>
        </div>
      ))}
    </div>
  );
}
