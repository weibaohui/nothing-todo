import { useState } from 'react';
import { Modal, Input, ColorPicker, Button, message } from 'antd';
import { useApp } from '../hooks/useApp';
import * as db from '../utils/database';

interface CreateTagModalProps {
  open: boolean;
  onClose: () => void;
}

export function CreateTagModal({ open, onClose }: CreateTagModalProps) {
  const { dispatch } = useApp();
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
      onClose();
    } catch (error) {
      message.error('创建失败: ' + error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title="创建标签"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="cancel" onClick={onClose}>取消</Button>,
        <Button key="create" type="primary" loading={loading} onClick={handleCreate}>创建</Button>,
      ]}
    >
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 8 }}>标签名称</div>
        <Input
          value={name}
          onChange={e => setName(e.target.value)}
          placeholder="输入标签名称"
        />
      </div>
      <div>
        <div style={{ marginBottom: 8 }}>标签颜色</div>
        <ColorPicker
          value={color}
          onChange={(_, hex) => setColor(hex)}
          showText
        />
      </div>
    </Modal>
  );
}
