import { useState, useEffect } from 'react';
import {
  Tabs,
  Form,
  Input,
  InputNumber,
  Select,
  Button,
  message,
  List,
  Popconfirm,
  ColorPicker,
  Upload,
  Empty,
  Card,
  Space,
  Typography,
  Spin,
  Modal,
  Table,
  Tag as AntTag,
} from 'antd';
import {
  SettingOutlined,
  CodeOutlined,
  TagOutlined,
  SaveOutlined,
  DownloadOutlined,
  DeleteOutlined,
  InboxOutlined,
} from '@ant-design/icons';
import { useApp } from '../hooks/useApp';
import * as db from '../utils/database';
import type { Config, ExecutorPaths } from '../types';
import yaml from 'js-yaml';

const { Paragraph } = Typography;
const { Dragger } = Upload;
const { Option } = Select;

const LOG_LEVELS = ['DEBUG', 'INFO', 'WARN', 'ERROR'];

const DEFAULT_EXECUTORS: ExecutorPaths = {
  opencode: 'opencode',
  hermes: 'hermes',
  joinai: 'joinai',
  claude_code: 'claude',
  codebuddy: 'codebuddy',
  kimi: 'kimi',
  atomcode: 'atomcode',
  codex: 'codex',
};

const EXECUTOR_KEYS: (keyof ExecutorPaths)[] = [
  'opencode',
  'hermes',
  'joinai',
  'claude_code',
  'codebuddy',
  'kimi',
  'atomcode',
  'codex',
];

const EXECUTOR_LABELS: Record<string, string> = {
  opencode: 'Opencode',
  hermes: 'Hermes',
  joinai: 'JoinAI',
  claude_code: 'Claude Code',
  codebuddy: 'CodeBuddy',
  kimi: 'Kimi',
  atomcode: 'AtomCode',
  codex: 'Codex',
};

export function SettingsPage() {
  const { state, dispatch } = useApp();
  const { tags } = state;

  const [configForm] = Form.useForm();
  const [configLoading, setConfigLoading] = useState(false);
  const [configSaving, setConfigSaving] = useState(false);

  const [tagName, setTagName] = useState('');
  const [tagColor, setTagColor] = useState('#0891b2');
  const [tagCreating, setTagCreating] = useState(false);

  const [importing, setImporting] = useState(false);

  // Import wizard state
  const [wizardOpen, setWizardOpen] = useState(false);
  const [wizardItems, setWizardItems] = useState<ImportItem[]>([]);
  const [wizardTags, setWizardTags] = useState<{ name: string; color: string }[]>([]);
  const [selectedRowKeys, setSelectedRowKeys] = useState<number[]>([]);

  interface BackupDataYaml {
    version: string;
    created_at: string;
    tags: { name: string; color: string }[];
    todos: {
      title: string;
      prompt: string;
      status: string;
      executor?: string;
      scheduler_enabled: boolean;
      scheduler_config?: string;
      tag_names: string[];
      workspace?: string;
    }[];
  }

  interface ImportItem {
    key: number;
    title: string;
    prompt: string;
    status: string;
    executor?: string;
    scheduler_enabled: boolean;
    scheduler_config?: string;
    tag_names: string[];
    workspace?: string;
    action: 'new' | 'overwrite';
    existingTitle?: string;
  }

  // Load config on mount
  useEffect(() => {
    setConfigLoading(true);
    db.getConfig()
      .then((cfg) => {
        configForm.setFieldsValue(cfg);
      })
      .catch((err) => {
        message.error('加载配置失败: ' + (err?.message || String(err)));
      })
      .finally(() => setConfigLoading(false));
  }, [configForm]);

  const handleSaveConfig = async () => {
    try {
      const values = await configForm.validateFields();
      setConfigSaving(true);
      await db.updateConfig(values as Config);
      message.success('配置已保存');
    } catch (err: any) {
      if (err?.errorFields) return; // validation error
      message.error('保存失败: ' + (err?.message || String(err)));
    } finally {
      setConfigSaving(false);
    }
  };

  const handleCreateTag = async () => {
    if (tagCreating) return;
    const name = tagName.trim();
    if (!name) {
      message.error('请输入标签名称');
      return;
    }
    setTagCreating(true);
    try {
      const newTag = await db.createTag(name, tagColor);
      dispatch({ type: 'ADD_TAG', payload: newTag });
      message.success('标签创建成功');
      setTagName('');
      setTagColor('#0891b2');
    } catch (err: any) {
      message.error('创建失败: ' + (err?.message || String(err)));
    } finally {
      setTagCreating(false);
    }
  };

  const handleDeleteTag = async (tagId: number) => {
    try {
      await db.deleteTag(tagId);
      dispatch({ type: 'DELETE_TAG', payload: tagId });
      message.success('标签已删除');
    } catch (err: any) {
      message.error('删除失败: ' + (err?.message || String(err)));
    }
  };

  const handleExportBackup = async () => {
    try {
      const response = await fetch('/xyz/backup/export', {
        headers: { Accept: 'application/x-yaml' },
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      const yamlText = await response.text();
      const blob = new Blob([yamlText], { type: 'application/x-yaml' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
      a.download = `aietodo-backup-${timestamp}.yaml`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      message.success('备份导出成功');
    } catch (err: any) {
      message.error(err?.message || '导出失败');
    }
  };

  const handleImportFile = async (file: File) => {
    const text = await file.text();
    try {
      const data = yaml.load(text) as BackupDataYaml;
      if (!data.todos || data.todos.length === 0) {
        message.error('备份文件中没有 Todo 数据');
        return false;
      }

      // 获取现有 todos 用于对比
      const existingTodos = await db.getAllTodos();
      const existingSet = new Set(existingTodos.map(t => `${t.title}\n${t.prompt}`));

      // 构建导入列表
      const items: ImportItem[] = data.todos.map((todo, idx) => {
        const key = `${todo.title}\n${todo.prompt}`;
        const exists = existingSet.has(key);
        const existing = exists ? existingTodos.find(t => `${t.title}\n${t.prompt}` === key) : undefined;
        return {
          key: idx,
          title: todo.title,
          prompt: todo.prompt,
          status: todo.status,
          executor: todo.executor,
          scheduler_enabled: todo.scheduler_enabled,
          scheduler_config: todo.scheduler_config,
          tag_names: todo.tag_names || [],
          workspace: todo.workspace,
          action: exists ? 'overwrite' as const : 'new' as const,
          existingTitle: existing?.title,
        };
      });

      setWizardTags(data.tags || []);
      setWizardItems(items);
      setSelectedRowKeys(items.map(i => i.key));
      setWizardOpen(true);
    } catch (err: any) {
      message.error('解析文件失败: ' + (err?.message || String(err)));
    }
    return false;
  };

  const handleWizardConfirm = async () => {
    if (selectedRowKeys.length === 0) {
      message.warning('请至少选择一项');
      return;
    }
    setImporting(true);
    try {
      const selectedTodos = wizardItems
        .filter(item => selectedRowKeys.includes(item.key))
        .map(({ key, action, existingTitle, ...todo }) => todo);
      const msg = await db.mergeBackup(wizardTags, selectedTodos);
      message.success(msg);
      setWizardOpen(false);
      window.location.reload();
    } catch (err: any) {
      message.error(err?.message || '导入失败');
    } finally {
      setImporting(false);
    }
  };

  const tabItems = [
    {
      key: 'system',
      label: (
        <span>
          <SettingOutlined style={{ marginRight: 6 }} />
          系统设置
        </span>
      ),
      children: (
        <Spin spinning={configLoading}>
          <Form
            form={configForm}
            layout="vertical"
            style={{ maxWidth: 600 }}
            initialValues={{
              port: 8088,
              host: '0.0.0.0',
              db_path: '~/.ntd/data.db',
              log_level: 'INFO',
            }}
          >
            <Form.Item
              name="port"
              label="服务端口"
              rules={[{ required: true, type: 'integer', min: 1, max: 65535 }]}
            >
              <InputNumber style={{ width: '100%' }} placeholder="8088" />
            </Form.Item>
            <Form.Item
              name="host"
              label="服务地址"
              rules={[{ required: true }]}
            >
              <Input placeholder="0.0.0.0" />
            </Form.Item>
            <Form.Item
              name="db_path"
              label="数据库路径"
              rules={[{ required: true }]}
            >
              <Input placeholder="~/.ntd/data.db" />
            </Form.Item>
            <Form.Item
              name="log_level"
              label="日志级别"
              rules={[{ required: true }]}
            >
              <Select placeholder="选择日志级别">
                {LOG_LEVELS.map((level) => (
                  <Option key={level} value={level}>
                    {level}
                  </Option>
                ))}
              </Select>
            </Form.Item>
            <Form.Item>
              <Button
                type="primary"
                onClick={handleSaveConfig}
                loading={configSaving}
                disabled={configLoading}
              >
                保存配置
              </Button>
            </Form.Item>
          </Form>
        </Spin>
      ),
    },
    {
      key: 'executors',
      label: (
        <span>
          <CodeOutlined style={{ marginRight: 6 }} />
          执行器路径
        </span>
      ),
      children: (
        <Spin spinning={configLoading}>
          <Form form={configForm} layout="vertical" style={{ maxWidth: 600 }}>
            <Paragraph type="secondary" style={{ marginBottom: 16 }}>
              配置各执行器的二进制路径。留空或填写命令名表示通过 PATH 查找。
            </Paragraph>
            {EXECUTOR_KEYS.map((key) => (
              <Form.Item
                key={key}
                name={['executors', key]}
                label={EXECUTOR_LABELS[key]}
              >
                <Input placeholder={DEFAULT_EXECUTORS[key]} />
              </Form.Item>
            ))}
            <Form.Item>
              <Button
                type="primary"
                onClick={handleSaveConfig}
                loading={configSaving}
                disabled={configLoading}
              >
                保存配置
              </Button>
            </Form.Item>
          </Form>
        </Spin>
      ),
    },
    {
      key: 'tags',
      label: (
        <span>
          <TagOutlined style={{ marginRight: 6 }} />
          标签管理
        </span>
      ),
      children: (
        <div style={{ maxWidth: 600 }}>
          <Card
            title="创建新标签"
            size="small"
            style={{ marginBottom: 24 }}
          >
            <Space direction="vertical" style={{ width: '100%' }}>
              <Input
                value={tagName}
                onChange={(e) => setTagName(e.target.value)}
                placeholder="输入标签名称"
                onPressEnter={handleCreateTag}
              />
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <ColorPicker
                  value={tagColor}
                  onChange={(_, hex) => setTagColor(hex)}
                  showText
                />
                <Button
                  type="primary"
                  loading={tagCreating}
                  onClick={handleCreateTag}
                >
                  创建标签
                </Button>
              </div>
            </Space>
          </Card>

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
                    borderRadius: 6,
                    marginBottom: 8,
                    border: '1px solid var(--color-border-light)',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10, flex: 1 }}>
                    <span
                      style={{
                        width: 16,
                        height: 16,
                        borderRadius: '50%',
                        backgroundColor: tag.color,
                        flexShrink: 0,
                      }}
                    />
                    <span style={{ fontSize: 14, fontWeight: 500 }}>{tag.name}</span>
                  </div>
                  <Popconfirm
                    title="删除标签"
                    description={`确定要删除标签 "${tag.name}" 吗？`}
                    onConfirm={() => handleDeleteTag(tag.id)}
                  >
                    <Button type="text" danger icon={<DeleteOutlined />} size="small" />
                  </Popconfirm>
                </List.Item>
              )}
            />
          )}
        </div>
      ),
    },
    {
      key: 'backup',
      label: (
        <span>
          <SaveOutlined style={{ marginRight: 6 }} />
          备份与恢复
        </span>
      ),
      children: (
        <div style={{ maxWidth: 600 }}>
          <Card title="导出备份" size="small" style={{ marginBottom: 24 }}>
            <Space direction="vertical" style={{ width: '100%' }}>
              <Paragraph type="secondary">
                将所有 Todo 和标签导出为 YAML 文件，方便迁移和存档
              </Paragraph>
              <Button
                type="primary"
                icon={<DownloadOutlined />}
                onClick={handleExportBackup}
                block
              >
                导出为 YAML 文件
              </Button>
            </Space>
          </Card>

          <Card title="导入备份" size="small">
            <Space direction="vertical" style={{ width: '100%' }}>
              <Paragraph type="secondary">
                从 YAML 文件恢复数据，支持预览和选择性导入
              </Paragraph>
              <Dragger
                accept=".yaml,.yml"
                beforeUpload={handleImportFile}
                showUploadList={false}
                disabled={importing}
                style={{ borderRadius: 12 }}
              >
                <p className="ant-upload-drag-icon">
                  <InboxOutlined style={{ color: '#0891b2' }} />
                </p>
                <p className="ant-upload-text">点击或拖拽 YAML 文件到此处</p>
                <p className="ant-upload-hint">将解析文件并展示预览，可选择性导入</p>
              </Dragger>
            </Space>
          </Card>
        </div>
      ),
    },
  ];

  return (
    <div
      style={{
        height: '100%',
        overflowY: 'auto',
        padding: '24px 32px',
        background: 'var(--color-bg-layout, #f8fafc)',
      }}
    >
      <div style={{ marginBottom: 24 }}>
        <h2 style={{ margin: 0, fontSize: 22, fontWeight: 700 }}>配置管理</h2>
        <Paragraph type="secondary" style={{ marginTop: 4 }}>
          管理系统配置、执行器路径、标签和备份
        </Paragraph>
      </div>
      <Tabs items={tabItems} type="card" />

      <Modal
        title="导入预览"
        open={wizardOpen}
        onCancel={() => setWizardOpen(false)}
        onOk={handleWizardConfirm}
        okText={`导入 ${selectedRowKeys.length} 项`}
        cancelText="取消"
        confirmLoading={importing}
        width={800}
        okButtonProps={{ disabled: selectedRowKeys.length === 0 }}
      >
        <div style={{ marginBottom: 12, display: 'flex', gap: 16 }}>
          <AntTag color="green">{wizardItems.filter(i => i.action === 'new').length} 个新建</AntTag>
          <AntTag color="orange">{wizardItems.filter(i => i.action === 'overwrite').length} 个覆盖</AntTag>
          <AntTag color="blue">已选 {selectedRowKeys.length} 项</AntTag>
        </div>
        <Table
          dataSource={wizardItems}
          rowKey="key"
          size="small"
          pagination={false}
          scroll={{ y: 400 }}
          rowSelection={{
            selectedRowKeys,
            onChange: (keys) => setSelectedRowKeys(keys as number[]),
          }}
          columns={[
            {
              title: '标题',
              dataIndex: 'title',
              ellipsis: true,
              width: '35%',
            },
            {
              title: '状态',
              dataIndex: 'action',
              width: 80,
              render: (action: 'new' | 'overwrite') => (
                <AntTag color={action === 'new' ? 'green' : 'orange'}>
                  {action === 'new' ? '新建' : '覆盖'}
                </AntTag>
              ),
            },
            {
              title: '执行器',
              dataIndex: 'executor',
              width: 100,
              render: (v: string | undefined) => v || '-',
            },
            {
              title: '标签',
              dataIndex: 'tag_names',
              width: 150,
              render: (names: string[]) => names.length > 0
                ? names.slice(0, 3).map(n => <AntTag key={n}>{n}</AntTag>)
                : '-',
            },
            {
              title: 'Prompt 摘要',
              dataIndex: 'prompt',
              ellipsis: true,
              render: (v: string) => v ? v.slice(0, 60) + (v.length > 60 ? '...' : '') : '-',
            },
          ]}
        />
      </Modal>
    </div>
  );
}
