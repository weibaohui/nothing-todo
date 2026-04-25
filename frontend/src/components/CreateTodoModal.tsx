import { useState } from 'react';
import { Modal, Input, Button, message } from 'antd';
import { useApp } from '../hooks/useApp';
import { TagCheckCardGroup } from './TagCheckCard';
import * as db from '../utils/database';

const { TextArea } = Input;

interface CreateTodoModalProps {
  open: boolean;
  onClose: () => void;
}

export function CreateTodoModal({ open, onClose }: CreateTodoModalProps) {
  const { dispatch, state } = useApp();
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [selectedTags, setSelectedTags] = useState<number[]>([]);
  const [loading, setLoading] = useState(false);

  const handleCreate = async () => {
    if (!title.trim()) {
      message.error('请输入 Todo 标题');
      return;
    }

    setLoading(true);
    try {
      const id = await db.createTodo(title.trim(), description.trim(), selectedTags);
      const newTodo = {
        id,
        title: title.trim(),
        description: description.trim(),
        status: 'pending' as const,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        deleted_at: null,
        tag_ids: selectedTags,
      };
      dispatch({ type: 'ADD_TODO', payload: newTodo });

      message.success('Todo 创建成功');
      setTitle('');
      setDescription('');
      setSelectedTags([]);
      onClose();
    } catch (error) {
      message.error('创建失败: ' + error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title="创建 Todo"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="cancel" onClick={onClose}>取消</Button>,
        <Button key="create" type="primary" loading={loading} onClick={handleCreate}>创建</Button>,
      ]}
    >
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 8 }}>标题 <span style={{ color: '#ff4d4f' }}>*</span></div>
        <Input
          value={title}
          onChange={e => setTitle(e.target.value)}
          placeholder="输入 Todo 标题"
        />
      </div>
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 8 }}>描述</div>
        <TextArea
          value={description}
          onChange={e => setDescription(e.target.value)}
          rows={4}
          placeholder="输入描述（会作为任务执行的内容）"
        />
      </div>
      {state.tags.length > 0 && (
        <div style={{ marginTop: 16 }}>
          <div style={{ marginBottom: 10, fontWeight: 600 }}>标签</div>
          <TagCheckCardGroup
            tags={state.tags}
            value={selectedTags}
            onChange={(val) => setSelectedTags((val as number[] | null) || [])}
            multiple
          />
        </div>
      )}
    </Modal>
  );
}
