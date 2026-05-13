import { useState, useEffect, useMemo } from 'react';
import { Modal, Input, Button, App, Empty, Card, Spin, Tag } from 'antd';
import { FileTextOutlined, SearchOutlined } from '@ant-design/icons';
import { useApp } from '../hooks/useApp';
import { TagCheckCardGroup } from './TagCheckCard';
import * as db from '../utils/database';
import type { TodoTemplate } from '../types';

const { TextArea } = Input;
const { Search } = Input;

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
  const [searchText, setSearchText] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);

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

  // Get unique categories
  const categories = useMemo(() => {
    const cats = Array.from(new Set(templates.map(t => t.category))).filter(c => c);
    return cats.sort();
  }, [templates]);

  // Filter templates by search and category
  const filteredTemplates = useMemo(() => {
    let result = templates;
    if (selectedCategory) {
      result = result.filter(t => t.category === selectedCategory);
    }
    if (searchText.trim()) {
      const search = searchText.toLowerCase();
      result = result.filter(t =>
        t.title.toLowerCase().includes(search) ||
        (t.prompt?.toLowerCase().includes(search))
      );
    }
    return result;
  }, [templates, selectedCategory, searchText]);

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
        title={
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <FileTextOutlined style={{ color: 'var(--color-primary)' }} />
            <span>选择模板</span>
          </div>
        }
        open={templateModalOpen}
        onCancel={() => {
          setTemplateModalOpen(false);
          setSearchText('');
          setSelectedCategory(null);
        }}
        footer={null}
        width={900}
        className="template-modal"
      >
        <div className="template-selector">
          {/* Search bar */}
          <div className="template-search">
            <Search
              placeholder="搜索模板标题或内容..."
              allowClear
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
              style={{ width: '100%' }}
              size="large"
              prefix={<SearchOutlined style={{ color: 'var(--color-text-tertiary)' }} />}
            />
          </div>

          {/* Content area */}
          <div className="template-content">
            {/* Categories sidebar */}
            <div className="template-categories">
              <div
                className={`template-category-item ${!selectedCategory ? 'active' : ''}`}
                onClick={() => setSelectedCategory(null)}
              >
                <span>全部模板</span>
                <Tag style={{ marginLeft: 'auto' }}>{templates.length}</Tag>
              </div>
              {categories.map(category => {
                const count = templates.filter(t => t.category === category).length;
                return (
                  <div
                    key={category}
                    className={`template-category-item ${selectedCategory === category ? 'active' : ''}`}
                    onClick={() => setSelectedCategory(category)}
                  >
                    <span>{category}</span>
                    <Tag style={{ marginLeft: 'auto' }}>{count}</Tag>
                  </div>
                );
              })}
            </div>

            {/* Templates list */}
            <div className="template-list">
              <Spin spinning={templatesLoading}>
                {filteredTemplates.length === 0 ? (
                  <Empty
                    description={searchText ? "未找到匹配的模板" : "暂无模板，请在设置中添加"}
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                  />
                ) : (
                  <div className="template-cards">
                    {filteredTemplates.map(template => (
                      <Card
                        key={template.id}
                        size="small"
                        className="template-card"
                        onClick={() => selectTemplate(template)}
                        hoverable
                      >
                        <div className="template-card-header">
                          <span className="template-card-title">{template.title}</span>
                          {template.is_system && <Tag color="blue" style={{ marginLeft: 8 }}>系统</Tag>}
                        </div>
                        <div className="template-card-desc">
                          {template.prompt || '(无内容)'}
                        </div>
                        <div className="template-card-footer">
                          <Tag>{template.category}</Tag>
                        </div>
                      </Card>
                    ))}
                  </div>
                )}
              </Spin>
            </div>
          </div>
        </div>
      </Modal>

      <style>{`
        .template-selector {
          display: flex;
          flex-direction: column;
          gap: 16px;
          min-height: 400px;
          max-height: 70vh;
        }

        .template-search {
          flex-shrink: 0;
        }

        .template-content {
          display: flex;
          gap: 16px;
          flex: 1;
          min-height: 0;
          overflow: hidden;
        }

        .template-categories {
          width: 180px;
          flex-shrink: 0;
          display: flex;
          flex-direction: column;
          gap: 4px;
          border-right: 1px solid var(--color-border-light);
          padding-right: 16px;
          overflow-y: auto;
        }

        .template-category-item {
          display: flex;
          align-items: center;
          padding: 10px 12px;
          border-radius: 8px;
          cursor: pointer;
          transition: all 0.2s ease;
          color: var(--color-text-secondary);
          font-size: 14px;
        }

        .template-category-item:hover {
          background: var(--color-bg-hover);
          color: var(--color-text);
        }

        .template-category-item.active {
          background: var(--color-primary-bg);
          color: var(--color-primary);
          font-weight: 600;
        }

        .template-list {
          flex: 1;
          overflow-y: auto;
          padding-left: 16px;
        }

        .template-cards {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 12px;
        }

        .template-card {
          cursor: pointer;
          transition: all 0.2s ease;
          border: 1px solid var(--color-border-light);
        }

        .template-card:hover {
          border-color: var(--color-primary);
          box-shadow: var(--shadow-primary);
          transform: translateY(-2px);
        }

        .template-card-header {
          display: flex;
          align-items: center;
          margin-bottom: 8px;
        }

        .template-card-title {
          font-weight: 600;
          font-size: 14px;
          color: var(--color-text);
        }

        .template-card-desc {
          font-size: 12px;
          color: var(--color-text-secondary);
          line-height: 1.5;
          max-height: 60px;
          overflow: hidden;
          text-overflow: ellipsis;
          display: -webkit-box;
          -webkit-line-clamp: 3;
          -webkit-box-orient: vertical;
          word-break: break-word;
        }

        .template-card-footer {
          margin-top: 8px;
          display: flex;
          justify-content: space-between;
          align-items: center;
        }

        /* Mobile responsive */
        @media (max-width: 768px) {
          .template-content {
            flex-direction: column;
          }

          .template-categories {
            width: 100%;
            flex-direction: row;
            flex-wrap: wrap;
            gap: 8px;
            border-right: none;
            border-bottom: 1px solid var(--color-border-light);
            padding-right: 0;
            padding-bottom: 16px;
            overflow-x: auto;
          }

          .template-category-item {
            padding: 6px 12px;
            white-space: nowrap;
          }

          .template-list {
            padding-left: 0;
            padding-top: 16px;
          }

          .template-cards {
            grid-template-columns: 1fr;
          }
        }

        /* Desktop - larger modal */
        @media (min-width: 769px) {
          .template-selector {
            min-height: 500px;
          }

          .template-cards {
            grid-template-columns: repeat(2, 1fr);
            gap: 16px;
          }

          .template-card {
            padding: 4px;
          }
        }
      `}</style>
    </>
  );
}
