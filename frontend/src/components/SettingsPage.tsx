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

const { Text, Paragraph } = Typography;
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
    setImporting(true);
    try {
      const msg = await db.importBackup(text);
      message.success(msg);
      window.location.reload();
    } catch (err: any) {
      message.error(err?.message || '导入失败');
    } finally {
      setImporting(false);
    }
    return false;
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
                从 YAML 文件恢复数据
                <Text type="danger" style={{ display: 'block', marginTop: 4 }}>
                  导入将清空当前所有数据，操作不可逆！
                </Text>
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
                <p className="ant-upload-hint">导入将清空当前所有数据</p>
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
    </div>
  );
}
