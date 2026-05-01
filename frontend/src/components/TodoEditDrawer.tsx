import { useEffect, useState } from 'react';
import { Drawer, Input, Button, App } from 'antd';
import { ReactMdeEditor } from './ReactMdeEditor';
import type { Todo } from '../types';

interface TodoEditDrawerProps {
  open: boolean;
  todo: Todo;
  onClose: () => void;
  onSave: (title: string, prompt: string) => Promise<void>;
}

export function TodoEditDrawer({ open, todo, onClose, onSave }: TodoEditDrawerProps) {
  const { message } = App.useApp();

  const [title, setTitle] = useState('');
  const [prompt, setPrompt] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open && todo) {
      setTitle(todo.title || '');
      setPrompt(todo.prompt || '');
    }
  }, [open, todo?.title, todo?.prompt]);

  const handleSave = async () => {
    if (!title.trim()) {
      message.warning('请输入任务标题');
      return;
    }
    setSaving(true);
    try {
      await onSave(title, prompt);
      message.success('更新成功');
      onClose();
    } catch (err) {
      message.error('保存失败: ' + err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Drawer
      open={open}
      onClose={onClose}
      title={null}
      closable={false}
      width="100%"
      placement="right"
      styles={{
        body: { padding: 0, display: 'flex', flexDirection: 'column' },
        wrapper: {},
      }}
      destroyOnClose
    >
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        background: 'var(--color-bg-elevated)',
      }}>
        {/* Header */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '12px 20px',
          borderBottom: '1px solid var(--color-border-light)',
          flexShrink: 0,
        }}>
          <div style={{ fontSize: 16, fontWeight: 700, color: 'var(--color-text)' }}>
            编辑任务
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <Button onClick={onClose}>取消</Button>
            <Button type="primary" onClick={handleSave} loading={saving}>
              保存
            </Button>
          </div>
        </div>

        {/* Title input */}
        <div style={{ padding: '12px 20px 0', flexShrink: 0 }}>
          <Input
            value={title}
            onChange={e => setTitle(e.target.value)}
            placeholder="任务标题"
            style={{
              fontSize: 15,
              fontWeight: 600,
              padding: '8px 12px',
            }}
          />
        </div>

        {/* Editor */}
        <div style={{ flex: 1, padding: '8px 20px 20px', overflow: 'hidden' }}>
          <ReactMdeEditor
            value={prompt}
            onChange={setPrompt}
            height={400}
          />
        </div>
      </div>
    </Drawer>
  );
}
