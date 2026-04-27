import { useEffect, useRef, useId, useState } from 'react';
import { Modal, Input, Button, App } from 'antd';
import Cherry from 'cherry-markdown';
import 'cherry-markdown/dist/cherry-markdown.css';
import { TagCheckCardGroup } from './TagCheckCard';
import type { Todo } from '../types';

interface TodoEditModalProps {
  open: boolean;
  todo: Todo;
  tags: Array<{ id: number; name: string; color: string }>;
  onClose: () => void;
  onSave: (title: string, prompt: string, tagIds: number[]) => Promise<void>;
}

export function TodoEditModal({ open, todo, tags, onClose, onSave }: TodoEditModalProps) {
  const { message } = App.useApp();
  const reactId = useId();
  const editorId = 'cherry-modal-' + reactId.replace(/:/g, '');
  const cherryRef = useRef<Cherry | null>(null);
  const isInternalUpdate = useRef(false);

  const [title, setTitle] = useState('');
  const [prompt, setPrompt] = useState('');
  const [selectedTags, setSelectedTags] = useState<number[]>([]);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open && todo) {
      setTitle(todo.title);
      setPrompt(todo.prompt || '');
      setSelectedTags((todo as any).tag_ids || []);
    }
  }, [open, todo]);

  useEffect(() => {
    if (!open) return;

    const timer = setTimeout(() => {
      const el = document.getElementById(editorId);
      if (!el) return;

      cherryRef.current = new Cherry({
        id: editorId,
        value: prompt || '',
        isPreviewOnly: false,
        editor: {
          defaultModel: 'edit&preview',
          height: 'calc(100vh - 320px)',
          codemirror: { placeholder: '输入 Prompt 内容...' },
        },
        toolbars: {
          toolbar: [
            'bold', 'italic', 'strikethrough', '|',
            'header', 'list', '|',
            'code', 'inlineCode', '|',
            'table', 'link', '|',
            'togglePreview',
          ],
          toolbarRight: [],
          bubble: [],
          float: [],
        },
        callback: {
          afterChange: (newVal: string) => {
            if (!isInternalUpdate.current) {
              setPrompt(newVal);
            }
          },
        },
      });
    }, 100);

    return () => {
      clearTimeout(timer);
      cherryRef.current?.destroy();
      cherryRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    const cherry = cherryRef.current;
    if (!cherry || !open) return;
    const currentVal = cherry.getMarkdown();
    if (currentVal !== prompt) {
      isInternalUpdate.current = true;
      cherry.setMarkdown(prompt || '');
      isInternalUpdate.current = false;
    }
  }, [prompt, open]);

  const handleSave = async () => {
    if (!title.trim()) {
      message.warning('请输入任务标题');
      return;
    }
    setSaving(true);
    try {
      await onSave(title, prompt, selectedTags);
      message.success('更新成功');
      onClose();
    } catch (err) {
      message.error('保存失败: ' + err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onCancel={onClose}
      title={null}
      footer={null}
      width="100vw"
      style={{ top: 0, maxWidth: '100vw', paddingBottom: 0 }}
      styles={{
        body: { padding: 0, height: '100vh', display: 'flex', flexDirection: 'column' },
        root: { borderRadius: 0 },
      }}
      destroyOnClose
      closable={false}
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
          <div style={{
            fontSize: 16,
            fontWeight: 700,
            color: 'var(--color-text)',
          }}>
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

        {/* Tags */}
        {tags.length > 0 && (
          <div style={{ padding: '12px 20px 0', flexShrink: 0 }}>
            <TagCheckCardGroup
              tags={tags}
              value={selectedTags[0] || null}
              onChange={(val) => setSelectedTags(val ? [val as number] : [])}
            />
          </div>
        )}

        {/* Editor */}
        <div style={{ flex: 1, padding: '8px 20px 20px', overflow: 'hidden' }}>
          <div id={editorId} style={{ height: '100%' }} />
        </div>
      </div>
    </Modal>
  );
}
