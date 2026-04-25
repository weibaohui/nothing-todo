import { useState } from 'react';
import { Modal, Input, ColorPicker, Button, App, List, Popconfirm, Empty } from 'antd';
import { useApp } from '../hooks/useApp';
import { DeleteOutlined } from '@ant-design/icons';
import * as db from '../utils/database';

interface CreateTagModalProps {
  open: boolean;
  onClose: () => void;
}

export function CreateTagModal({ open, onClose }: CreateTagModalProps) {
  const { state, dispatch } = useApp();
  const { message } = App.useApp();
  const { tags } = state;
  const [name, setName] = useState('');
  const [color, setColor] = useState('#1890ff');
  const [loading, setLoading] = useState(false);

  const handleCreate = async () => {
    if (!name.trim()) {
      message.error('请输入标签名称');
      return;
    }

    setLoading(true);
    try {
      const newTag = await db.createTag(name.trim(), color);
      dispatch({ type: 'ADD_TAG', payload: newTag });
      message.success('标签创建成功');
      setName('');
      setColor('#1890ff');
    } catch (error) {
      message.error('创建失败: ' + error);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (tagId: number) => {
    try {
      await db.deleteTag(tagId);
      dispatch({ type: 'DELETE_TAG', payload: tagId });
      message.success('标签删除成功');
    } catch (error) {
      message.error('删除失败: ' + error);
    }
  };

  return (
    <Modal
      title="标签管理"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="close" onClick={onClose}>关闭</Button>,
      ]}
      width={500}
    >
      {/* 创建新标签区域 */}
      <div style={{
        padding: '16px',
        background: 'var(--color-bg)',
        borderRadius: '8px',
        marginBottom: '20px',
        border: '1px solid var(--color-border)'
      }}>
        <div style={{ marginBottom: 12 }}>
          <div style={{ marginBottom: 8, fontWeight: 600 }}>创建新标签</div>
          <Input
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="输入标签名称"
            onPressEnter={handleCreate}
          />
        </div>
        <div style={{ marginBottom: 12 }}>
          <div style={{ marginBottom: 8, fontWeight: 600 }}>标签颜色</div>
          <ColorPicker
            value={color}
            onChange={(_, hex) => setColor(hex)}
            showText
          />
        </div>
        <Button
          type="primary"
          loading={loading}
          onClick={handleCreate}
          block
        >
          创建标签
        </Button>
      </div>

      {/* 现有标签列表 */}
      <div>
        <div style={{ marginBottom: 12, fontWeight: 600 }}>现有标签</div>
        {tags.length === 0 ? (
          <Empty description="暂无标签" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        ) : (
          <List
            dataSource={tags}
            renderItem={(tag) => (
              <List.Item
                style={{
                  padding: '10px 12px',
                  background: 'var(--color-bg)',
                  borderRadius: '6px',
                  marginBottom: '8px',
                  border: '1px solid var(--color-border-light)',
                  transition: 'all 0.2s',
                }}
              >
                <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '10px',
                  flex: 1
                }}>
                  <span
                    style={{
                      width: '16px',
                      height: '16px',
                      borderRadius: '50%',
                      backgroundColor: tag.color,
                      flexShrink: 0,
                    }}
                  />
                  <span style={{
                    fontSize: '14px',
                    fontWeight: 500,
                    color: 'var(--color-text)'
                  }}>
                    {tag.name}
                  </span>
                </div>
                <Popconfirm
                  title="删除标签"
                  description={`确定要删除标签 "${tag.name}" 吗？`}
                  onConfirm={() => handleDelete(tag.id)}
                  okText="确定"
                  cancelText="取消"
                >
                  <Button
                    type="text"
                    danger
                    icon={<DeleteOutlined />}
                    size="small"
                  />
                </Popconfirm>
              </List.Item>
            )}
          />
        )}
      </div>
    </Modal>
  );
}
