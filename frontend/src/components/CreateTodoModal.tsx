import { useState } from 'react';
import { Modal, Input, Button, App } from 'antd';
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
  const { message } = App.useApp();
  const [title, setTitle] = useState('');
  const [prompt, setPrompt] = useState('');
  const [selectedTag, setSelectedTag] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);

  const handleCreate = async () => {
    if (!title.trim()) {
      message.error('请输入 Todo 标题');
      return;
    }

    setLoading(true);
    try {
      const tagIds = selectedTag !== null ? [selectedTag] : [];
      const newTodo = await db.createTodo(title.trim(), prompt.trim(), tagIds);
      dispatch({ type: 'ADD_TODO', payload: newTodo });

      message.success('Todo 创建成功');
      setTitle('');
      setPrompt('');
      setSelectedTag(null);
      onClose();
    } catch (error) {
      message.error('创建失败: ' + (error instanceof Error ? error.message : String(error)));
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
        <div style={{ marginBottom: 8 }}>Prompt</div>
        <TextArea
          value={prompt}
          onChange={e => setPrompt(e.target.value)}
          rows={4}
          placeholder="输入 Prompt（会作为任务执行的内容，留空则使用标题）"
        />
      </div>
      {state.tags.length > 0 && (
        <div style={{ marginTop: 16 }}>
          <div style={{ marginBottom: 10, fontWeight: 600 }}>标签</div>
          <TagCheckCardGroup
            tags={state.tags}
            value={selectedTag}
            onChange={(val) => setSelectedTag(val as number | null)}
          />
        </div>
      )}
    </Modal>
  );
}
