import { useState, useEffect } from 'react';
import { Modal, Input, Button, App, Space, Empty, Card, List, Spin, Tag } from 'antd';
import { FileTextOutlined } from '@ant-design/icons';
import { useApp } from '../hooks/useApp';
import { TagCheckCardGroup } from './TagCheckCard';
import * as db from '../utils/database';
import type { TodoTemplate } from '../types';

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

  // Template selection modal state
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [templates, setTemplates] = useState<TodoTemplate[]>([]);
  const [templatesLoading, setTemplatesLoading] = useState(false);

  useEffect(() => {
    if (open && state.tags.length > 0) {
      setSelectedTag(null);
    }
  }, [open, state.tags.length]);

  const loadTemplates = () => {
    setTemplatesLoading(true);
    db.getTodoTemplates()
      .then(setTemplates)
      .catch(() => message.error('加载模板失败'))
      .finally(() => setTemplatesLoading(false));
  };

  const openTemplateModal = () => {
    loadTemplates();
    setTemplateModalOpen(true);
  };

  const selectTemplate = (template: TodoTemplate) => {
    setTitle(template.title);
    setPrompt(template.prompt || '');
    setTemplateModalOpen(false);
    message.success('已应用模板');
  };

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
    <>
      <Modal
        title="创建 Todo"
        open={open}
        onCancel={onClose}
        footer={[
          <Button key="cancel" onClick={onClose}>取消</Button>,
          <Button key="template" icon={<FileTextOutlined />} onClick={openTemplateModal}>从模板创建</Button>,
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

      <Modal
        title="选择模板"
        open={templateModalOpen}
        onCancel={() => setTemplateModalOpen(false)}
        footer={null}
        width={600}
      >
        <Spin spinning={templatesLoading}>
          {templates.length === 0 ? (
            <Empty description="暂无模板，请在设置中添加" />
          ) : (
            <Space direction="vertical" style={{ width: '100%' }}>
              {Array.from(new Set(templates.map(t => t.category))).sort().map(category => (
                <Card key={category} title={category || '未分类'} size="small">
                  <List
                    dataSource={templates.filter(t => t.category === category)}
                    renderItem={(template) => (
                      <List.Item style={{ overflow: 'hidden' }}>
                        <Button
                          type="text"
                          onClick={() => selectTemplate(template)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter' || e.key === ' ') {
                              e.preventDefault();
                              selectTemplate(template);
                            }
                          }}
                          style={{ width: '100%', height: 'auto', textAlign: 'left', padding: 0, wordBreak: 'break-word', overflowWrap: 'break-word' }}
                        >
                          <List.Item.Meta
                            title={
                              <span>
                                {template.title}
                                {template.is_system && <Tag color="blue" style={{ marginLeft: 8 }}>系统</Tag>}
                              </span>
                            }
                            description={template.prompt || '(无内容)'}
                          />
                        </Button>
                      </List.Item>
                    )}
                  />
                </Card>
              ))}
            </Space>
          )}
        </Spin>
      </Modal>
    </>
  );
}
