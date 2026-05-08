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
  Switch,
} from 'antd';
import {
  SettingOutlined,
  CodeOutlined,
  TagOutlined,
  SaveOutlined,
  DownloadOutlined,
  DeleteOutlined,
  InboxOutlined,
  DatabaseOutlined,
  ClockCircleOutlined,
  ThunderboltOutlined,
  InfoCircleOutlined,
  MessageOutlined,
  QrcodeOutlined,
  CopyOutlined,
  ReloadOutlined,
  PlusOutlined,
  HistoryOutlined,
} from '@ant-design/icons';
import { Cron } from 'react-js-cron';
import QRCode from 'qrcode';
import 'react-js-cron/dist/styles.css';
import { useApp } from '../hooks/useApp';
import * as db from '../utils/database';
import type { FeishuPushStatus } from '../utils/database';
import { CRON_ZH_LOCALE, cronTo5, cronTo6 } from '../utils/cron';
import type { Config, ExecutorPaths, FeishuHistoryMessage, FeishuHistoryChat } from '../types';
import yaml from 'js-yaml';
import { CronPresetSelect } from './CronPresetSelect';
import { SkillsPanel } from './SkillsPanel';

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

interface SettingsPageProps {
  onBack?: () => void;
}

export function SettingsPage({ onBack }: SettingsPageProps) {
  const { state, dispatch } = useApp();
  const { tags } = state;

  const [configForm] = Form.useForm();
  const [configLoading, setConfigLoading] = useState(false);
  const [configSaving, setConfigSaving] = useState(false);

  const [tagName, setTagName] = useState('');
  const [tagColor, setTagColor] = useState('#0891b2');
  const [tagCreating, setTagCreating] = useState(false);

  const [importing, setImporting] = useState(false);

  // Selective export state
  const [exportModalOpen, setExportModalOpen] = useState(false);
  const [exportTodoKeys, setExportTodoKeys] = useState<number[]>([]);
  const [exportingSelected, setExportingSelected] = useState(false);

  // Database backup state
  const [backupStatus, setBackupStatus] = useState<{
    auto_backup_enabled: boolean;
    auto_backup_cron: string;
    last_backup: string | null;
    files: { name: string; size: number; created_at: string }[];
  } | null>(null);
  const [autoBackupEnabled, setAutoBackupEnabled] = useState(false);
  const [autoBackupCron, setAutoBackupCron] = useState('0 0 3 * * *');
  const [backupLoading, setBackupLoading] = useState(false);

  // Version info state
  const [versionInfo, setVersionInfo] = useState<{ version: string; git_sha: string; git_describe: string } | null>(null);
  const [versionLoading, setVersionLoading] = useState(false);

  // Agent Bots state
  const [agentBots, setAgentBots] = useState<db.AgentBot[]>([]);
  const [botsLoading, setBotsLoading] = useState(false);
  const [feishuPushStatus, setFeishuPushStatus] = useState<FeishuPushStatus[]>([]);
  const [binding, setBinding] = useState(false);
  const [bindModalOpen, setBindModalOpen] = useState(false);
  const [qrCodeUrl, setQrCodeUrl] = useState('');
  const [pollError, setPollError] = useState('');
  const [bindSuccess, setBindSuccess] = useState(false);

  // Feishu history state
  const [historyMessages, setHistoryMessages] = useState<FeishuHistoryMessage[]>([]);
  const [historyChats, setHistoryChats] = useState<FeishuHistoryChat[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [historyTotal, setHistoryTotal] = useState(0);
  const [historyPage, setHistoryPage] = useState(1);
  const [historyPageSize, setHistoryPageSize] = useState(20);
  const [historySelectedChatId, setHistorySelectedChatId] = useState<string | undefined>(undefined);
  const [historyAddModalOpen, setHistoryAddModalOpen] = useState(false);
  const [historyForm] = Form.useForm();

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

  // Load database backup status
  useEffect(() => {
    db.getDatabaseBackupStatus()
      .then((status) => {
        setBackupStatus(status);
        setAutoBackupEnabled(status.auto_backup_enabled);
        setAutoBackupCron(status.auto_backup_cron);
      })
      .catch(() => {});
  }, []);

  // Load version info
  useEffect(() => {
    setVersionLoading(true);
    db.getVersion()
      .then((info) => {
        setVersionInfo(info);
      })
      .catch(() => {})
      .finally(() => setVersionLoading(false));
  }, []);

  // Load agent bots
  const loadAgentBots = () => {
    setBotsLoading(true);
    db.getAgentBots()
      .then((bots) => setAgentBots(bots))
      .catch(() => {})
      .finally(() => setBotsLoading(false));
  };

  const loadFeishuPush = () => {
    db.getFeishuPush()
      .then((status) => setFeishuPushStatus(status))
      .catch(() => {});
  };

  const loadHistoryMessages = async () => {
    setHistoryLoading(true);
    try {
      const data = await db.getFeishuHistoryMessages({
        chat_id: historySelectedChatId,
        page: historyPage,
        page_size: historyPageSize,
      });
      setHistoryMessages(data.messages);
      setHistoryTotal(data.total);
    } catch {
      message.error('加载历史消息失败');
    } finally {
      setHistoryLoading(false);
    }
  };

  const loadHistoryChats = async () => {
    try {
      const data = await db.getFeishuHistoryChats();
      setHistoryChats(data);
    } catch (e) {
      console.error('加载群聊配置失败', e);
    }
  };

  useEffect(() => {
    loadHistoryChats();
  }, []);

  useEffect(() => {
    loadHistoryMessages();
  }, [historyPage, historyPageSize, historySelectedChatId]);

  const handleAddHistoryChat = async () => {
    try {
      const values = await historyForm.validateFields();
      await db.createFeishuHistoryChat(values);
      message.success('添加成功');
      setHistoryAddModalOpen(false);
      historyForm.resetFields();
      loadHistoryChats();
    } catch (e) {
      if (e instanceof Error) {
        message.error(e.message);
      }
    }
  };

  useEffect(() => {
    loadAgentBots();
    loadFeishuPush();
  }, []);

  // 飞书绑定 — 后端轮询模式，前端只需一次调用
  const handleStartFeishuBind = async () => {
    setBinding(true);
    setBindSuccess(false);
    setPollError('');
    setQrCodeUrl('');
    setBindModalOpen(true);

    try {
      const initRes = await db.feishuInit();
      if (!initRes.supported) {
        setPollError('当前环境不支持 client_secret 认证');
        setBinding(false);
        return;
      }

      const beginRes = await db.feishuBegin();

      const qrDataUrl = await QRCode.toDataURL(beginRes.qr_url, {
        width: 256,
        margin: 2,
      });
      setQrCodeUrl(qrDataUrl);

      // 后端内部轮询，等待扫码结果
      const pollRes = await db.feishuPoll(beginRes.device_code, beginRes.interval, beginRes.expire_in);

      if (pollRes.success) {
        setBindSuccess(true);
        message.success(`绑定成功！Bot: ${pollRes.bot_name || 'Feishu Bot'}`);
        loadAgentBots();
        loadFeishuPush();
        setTimeout(() => {
          setBindModalOpen(false);
          setQrCodeUrl('');
        }, 2000);
      } else {
        const errMsg = pollRes.error === 'access_denied' ? '用户拒绝了绑定请求'
          : pollRes.error === 'expired_token' ? '二维码已过期，请重新绑定'
          : '绑定超时，请重试';
        setPollError(errMsg);
      }
    } catch (err: any) {
      setPollError(err?.message || '启动绑定失败');
    } finally {
      setBinding(false);
    }
  };

  const handleDeleteBot = async (botId: number) => {
    try {
      await db.deleteAgentBot(botId);
      message.success('已删除');
      loadAgentBots();
    } catch (err: any) {
      message.error(err?.message || '删除失败');
    }
  };

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

  // Selective export
  const handleExportSelected = async () => {
    if (exportTodoKeys.length === 0) {
      message.warning('请至少选择一项');
      return;
    }
    setExportingSelected(true);
    try {
      const response = await fetch('/xyz/backup/export-selected', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'application/x-yaml' },
        body: JSON.stringify({ todo_ids: exportTodoKeys }),
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
      a.download = `aietodo-backup-selected-${timestamp}.yaml`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      message.success(`已导出 ${exportTodoKeys.length} 项`);
      setExportModalOpen(false);
    } catch (err: any) {
      message.error(err?.message || '导出失败');
    } finally {
      setExportingSelected(false);
    }
  };

  // Database backup handlers
  const handleTriggerBackup = async () => {
    setBackupLoading(true);
    try {
      const msg = await db.triggerLocalBackup();
      message.success(msg);
      const status = await db.getDatabaseBackupStatus();
      setBackupStatus(status);
    } catch (err: any) {
      message.error(err?.message || '备份失败');
    } finally {
      setBackupLoading(false);
    }
  };

  const handleDownloadDatabase = async () => {
    try {
      const response = await fetch('/xyz/backup/database/download');
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
      a.download = `ntd-database-${timestamp}.db`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      message.success('数据库下载成功');
    } catch (err: any) {
      message.error(err?.message || '下载失败');
    }
  };

  const handleSaveAutoBackup = async () => {
    setBackupLoading(true);
    try {
      await db.updateAutoBackup(autoBackupEnabled, autoBackupCron);
      message.success('自动备份配置已保存');
    } catch (err: any) {
      message.error(err?.message || '保存失败');
    } finally {
      setBackupLoading(false);
    }
  };

  const handleDeleteBackup = async (filename: string) => {
    try {
      await db.deleteBackupFile(filename);
      message.success('已删除');
      const status = await db.getDatabaseBackupStatus();
      setBackupStatus(status);
    } catch (err: any) {
      message.error(err?.message || '删除失败');
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
                将 Todo 和标签导出为 YAML 文件，方便迁移和存档
              </Paragraph>
              <div style={{ display: 'flex', gap: 8 }}>
                <Button
                  type="primary"
                  icon={<DownloadOutlined />}
                  onClick={handleExportBackup}
                  style={{ flex: 1 }}
                >
                  导出全部
                </Button>
                <Button
                  icon={<DownloadOutlined />}
                  onClick={() => setExportModalOpen(true)}
                  style={{ flex: 1 }}
                >
                  选择性导出
                </Button>
              </div>
            </Space>
          </Card>

          <Card title="导入备份" size="small" style={{ marginBottom: 24 }}>
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

          <Card title="数据库备份" size="small">
            <Space direction="vertical" style={{ width: '100%' }} size="middle">
              <Paragraph type="secondary">
                直接备份 SQLite 数据库文件，包含所有数据（含执行记录）
              </Paragraph>
              <div style={{ display: 'flex', gap: 8 }}>
                <Button
                  icon={<DownloadOutlined />}
                  onClick={handleDownloadDatabase}
                >
                  下载数据库
                </Button>
                <Button
                  icon={<DatabaseOutlined />}
                  onClick={handleTriggerBackup}
                  loading={backupLoading}
                >
                  备份到服务器
                </Button>
              </div>

              <div style={{ borderTop: '1px solid var(--color-border-light)', paddingTop: 12, marginTop: 4 }}>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
                  <span style={{ fontWeight: 600 }}><ClockCircleOutlined style={{ marginRight: 6 }} />自动备份</span>
                  <Switch checked={autoBackupEnabled} onChange={setAutoBackupEnabled} />
                </div>
                {autoBackupEnabled && (
                  <CronPresetSelect
                    value={autoBackupCron}
                    onChange={(val) => setAutoBackupCron(val)}
                  />
                )}
                {autoBackupEnabled && (
                  <Cron
                    value={cronTo5(autoBackupCron)}
                    setValue={(val: string) => {
                      setAutoBackupCron(cronTo6(val));
                    }}
                    locale={CRON_ZH_LOCALE}
                    defaultPeriod="day"
                    humanizeLabels
                    allowClear={false}
                  />
                )}
                <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 8 }}>
                  <Button size="small" type="primary" onClick={handleSaveAutoBackup} loading={backupLoading}>
                    保存
                  </Button>
                </div>
              </div>

              {backupStatus && backupStatus.files.length > 0 && (
                <div style={{ borderTop: '1px solid var(--color-border-light)', paddingTop: 12 }}>
                  <div style={{ fontWeight: 600, marginBottom: 8 }}>备份文件 ({backupStatus.files.length})</div>
                  <List
                    size="small"
                    dataSource={backupStatus.files}
                    renderItem={(file) => (
                      <List.Item
                        style={{ padding: '6px 0', fontSize: 12 }}
                      >
                        <div>
                          <div style={{ fontWeight: 500 }}>{file.name}</div>
                          <div style={{ color: 'var(--color-text-tertiary)', fontSize: 11 }}>
                            {(file.size / 1024).toFixed(1)} KB · {file.created_at}
                          </div>
                        </div>
                        <Popconfirm title="确定删除此备份？" onConfirm={() => handleDeleteBackup(file.name)}>
                          <Button type="text" danger icon={<DeleteOutlined />} size="small" />
                        </Popconfirm>
                      </List.Item>
                    )}
                  />
                </div>
              )}
            </Space>
          </Card>
        </div>
      ),
    },
    {
      key: 'skills',
      label: (
        <span>
          <ThunderboltOutlined style={{ marginRight: 6 }} />
          Skills 管理
        </span>
      ),
      children: <SkillsPanel />,
    },
    {
      key: 'messages',
      label: (
        <span>
          <MessageOutlined style={{ marginRight: 6 }} />
          消息
        </span>
      ),
      children: (
        <Tabs
          defaultActiveKey="bind"
          size="small"
          items={[
            {
              key: 'bind',
              label: '绑定',
              children: (
                <div className="settings-messages-tab" style={{ maxWidth: 700 }}>
                  <Card
                    title="绑定消息接收智能体"
                    size="small"
                    style={{ marginBottom: 24 }}
                    extra={
                      <Button
                        type="primary"
                        icon={<QrcodeOutlined />}
                        onClick={handleStartFeishuBind}
                        loading={binding}
                        size="small"
                      >
                        绑定飞书
                      </Button>
                    }
                  >
                    <Paragraph type="secondary" style={{ marginBottom: 16, fontSize: 13 }}>
                      绑定飞书智能体 Bot 后，可以接收任务执行结果和通知消息。支持绑定多个 Bot。
                    </Paragraph>

                    <Spin spinning={botsLoading}>
                      {agentBots.length === 0 ? (
                        <Empty description="暂无绑定的智能体" image={Empty.PRESENTED_IMAGE_SIMPLE} />
                      ) : (
                        <List
                          dataSource={agentBots}
                          renderItem={(bot) => {
                            let botConfig: Record<string, boolean> = { dm_enabled: true, group_enabled: true, group_require_mention: true, echo_reply: true };
                            try { botConfig = JSON.parse(bot.config || '{}'); } catch {}
                            const isFeishu = bot.bot_type === 'feishu';
                            const handleConfigChange = async (key: string, val: boolean) => {
                              const newConfig = { ...botConfig, [key]: val };
                              try {
                                await db.updateAgentBotConfig(bot.id, JSON.stringify(newConfig));
                                setAgentBots(prev => prev.map(b => b.id === bot.id ? { ...b, config: JSON.stringify(newConfig) } : b));
                              } catch (e: any) {
                                message.error('保存配置失败: ' + (e.message || '未知错误'));
                              }
                            };

                            const botPushStatus = feishuPushStatus.find(p => p.bot_id === bot.id);
                            const hasPushTarget = !!botPushStatus && (botPushStatus.receive_id || botPushStatus.chat_id);
                            const handlePushLevelChange = async (level: db.FeishuPushLevel) => {
                              try {
                                await db.updateFeishuPush({ botId: bot.id, pushLevel: level });
                                loadFeishuPush();
                              } catch (e: any) {
                                message.error('设置推送失败: ' + (e.message || '未知错误'));
                              }
                            };
                            const handlePushTargetUpdate = async (field: 'receive_id' | 'receive_id_type' | 'chat_id', value: string) => {
                              try {
                                await db.updateFeishuPush({ botId: bot.id, [field]: value });
                                loadFeishuPush();
                              } catch (e: any) {
                                message.error('更新推送目标失败: ' + (e.message || '未知错误'));
                              }
                            };
                            const copyToClipboard = (text: string, label: string) => {
                              navigator.clipboard.writeText(text).then(() => {
                                message.success(`${label} 已复制`);
                              }).catch(() => {
                                message.error('复制失败');
                              });
                            };

                            return (
                              <div
                                key={bot.id}
                                style={{
                                  padding: '12px',
                                  background: 'var(--color-bg)',
                                  borderRadius: 8,
                                  marginBottom: 8,
                                  border: '1px solid var(--color-border-light)',
                                }}
                              >
                                <div style={{ display: 'flex', alignItems: 'flex-start', gap: 10 }}>
                                  <div
                                    style={{
                                      width: 36,
                                      height: 36,
                                      borderRadius: 8,
                                      background: isFeishu ? '#1976D2' : '#888',
                                      display: 'flex',
                                      alignItems: 'center',
                                      justifyContent: 'center',
                                      color: '#fff',
                                      fontWeight: 700,
                                      fontSize: 14,
                                      flexShrink: 0,
                                    }}
                                  >
                                    {isFeishu ? '飞' : '其他'}
                                  </div>
                                  <div style={{ flex: 1, minWidth: 0 }}>
                                    <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                                      <span style={{ fontWeight: 600, fontSize: 14, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{bot.bot_name}</span>
                                      <AntTag color={bot.enabled ? 'green' : 'default'} style={{ marginRight: 0 }}>
                                        {bot.enabled ? '已启用' : '已禁用'}
                                      </AntTag>
                                    </div>
                                    <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', wordBreak: 'break-all', lineHeight: 1.6 }}>
                                      App ID: {bot.app_id}
                                    </div>
                                    {bot.domain && (
                                      <div style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>
                                        平台: {bot.domain === 'lark' ? 'Lark 国际版' : '飞书'}
                                      </div>
                                    )}
                                    <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginTop: 2 }}>
                                      绑定时间: {new Date(bot.created_at).toLocaleString()}
                                    </div>
                                  </div>
                                  <Popconfirm
                                    title="删除确认"
                                    description={`确定要删除 "${bot.bot_name}" 吗？`}
                                    onConfirm={() => handleDeleteBot(bot.id)}
                                    okText="删除"
                                    cancelText="取消"
                                    okButtonProps={{ danger: true }}
                                  >
                                    <Button type="text" danger icon={<DeleteOutlined />} size="small" style={{ flexShrink: 0 }} />
                                  </Popconfirm>
                                </div>
                                {isFeishu && (
                                  <div style={{ marginTop: 8, paddingTop: 8, borderTop: '1px solid var(--color-border-light)' }}>
                                    <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginBottom: 6 }}>消息配置</div>
                                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px 16px' }}>
                                      <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12 }}>
                                        <Switch size="small" checked={botConfig.dm_enabled !== false} onChange={v => handleConfigChange('dm_enabled', v)} />
                                        接收单聊消息
                                      </span>
                                      <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12 }}>
                                        <Switch size="small" checked={botConfig.group_enabled !== false} onChange={v => handleConfigChange('group_enabled', v)} />
                                        接收群聊消息
                                      </span>
                                      <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12 }}>
                                        <Switch size="small" checked={botConfig.group_require_mention !== false} onChange={v => handleConfigChange('group_require_mention', v)} />
                                        群聊仅处理@
                                      </span>
                                      <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12 }}>
                                        <Switch size="small" checked={botConfig.echo_reply !== false} onChange={v => handleConfigChange('echo_reply', v)} />
                                        Echo 回复
                                      </span>
                                    </div>
                                    {hasPushTarget && (
                                      <div style={{ marginTop: 8 }}>
                                        <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginBottom: 6 }}>实时推送 {botPushStatus.receive_id_type === 'open_id' ? '(私聊)' : '(群聊)'}</div>
                                        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, alignItems: 'center', marginBottom: 6 }}>
                                          <Select
                                            size="small"
                                            value={botPushStatus.push_level}
                                            onChange={handlePushLevelChange}
                                            style={{ width: 100 }}
                                            options={[
                                              { value: 'disabled', label: '关闭' },
                                              { value: 'result_only', label: '仅结论' },
                                              { value: 'all', label: '全部' },
                                            ]}
                                          />
                                        </div>
                                        <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', marginBottom: 4 }}>推送目标信息（可编辑）</div>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                                          <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                                            <span style={{ fontSize: 11, width: 80, color: 'var(--color-text-tertiary)' }}>接收ID:</span>
                                            <Input
                                              size="small"
                                              value={botPushStatus.receive_id}
                                              onChange={(e) => handlePushTargetUpdate('receive_id', e.target.value)}
                                              style={{ flex: 1, fontSize: 11 }}
                                            />
                                            <Button size="small" icon={<CopyOutlined />} onClick={() => copyToClipboard(botPushStatus.receive_id, 'receive_id')} />
                                          </div>
                                          <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                                            <span style={{ fontSize: 11, width: 80, color: 'var(--color-text-tertiary)' }}>群ID:</span>
                                            <Input
                                              size="small"
                                              value={botPushStatus.chat_id || ''}
                                              onChange={(e) => handlePushTargetUpdate('chat_id', e.target.value)}
                                              style={{ flex: 1, fontSize: 11 }}
                                            />
                                            <Button size="small" icon={<CopyOutlined />} onClick={() => copyToClipboard(botPushStatus.chat_id || '', 'chat_id')} />
                                          </div>
                                          <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                                            <span style={{ fontSize: 11, width: 80, color: 'var(--color-text-tertiary)' }}>推送类型:</span>
                                            <Select
                                              size="small"
                                              value={botPushStatus.receive_id_type}
                                              onChange={(v) => handlePushTargetUpdate('receive_id_type', v)}
                                              style={{ width: 100 }}
                                              options={[
                                                { value: 'open_id', label: '私聊' },
                                                { value: 'chat_id', label: '群聊' },
                                              ]}
                                            />
                                            <Button size="small" icon={<CopyOutlined />} onClick={() => copyToClipboard(botPushStatus.receive_id_type, 'receive_id_type')} />
                                          </div>
                                        </div>
                                      </div>
                                    )}
                                  </div>
                                )}
                              </div>
                            );
                          }}
                        />
                      )}
                    </Spin>
                  </Card>

                  <Modal
                    title={
                      <Space>
                        <QrcodeOutlined />
                        绑定飞书智能体
                      </Space>
                    }
                    open={bindModalOpen}
                    onCancel={() => {
                      setBindModalOpen(false);
                      setQrCodeUrl('');
                      setPollError('');
                      setBindSuccess(false);
                    }}
                    footer={null}
                    width={400}
                    centered
                    className="settings-bind-modal"
                  >
                    <div style={{ textAlign: 'center', padding: '16px 0' }}>
                      {pollError && (
                        <div style={{ marginBottom: 16, color: '#ff4d4f', fontSize: 13 }}>
                          {pollError}
                        </div>
                      )}

                      {bindSuccess ? (
                        <div style={{ color: '#52c41a', fontSize: 48, marginBottom: 16 }}>
                          ✓
                        </div>
                      ) : (
                        <>
                          {qrCodeUrl ? (
                            <div style={{ marginBottom: 16 }}>
                              <img src={qrCodeUrl} alt="QR Code" style={{ width: '100%', maxWidth: 200, height: 'auto' }} />
                              <div style={{ marginTop: 12, color: 'var(--color-text-secondary)', fontSize: 13 }}>
                                请使用飞书 App 扫描二维码绑定
                              </div>
                              <div style={{ marginTop: 6, fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                                二维码有效期 10 分钟，请尽快完成
                              </div>
                            </div>
                          ) : (
                            <Spin size="large" />
                          )}
                        </>
                      )}

                      {binding && !qrCodeUrl && (
                        <div style={{ marginTop: 16, color: 'var(--color-text-secondary)', fontSize: 13 }}>
                          正在生成二维码...
                        </div>
                      )}
                    </div>
                  </Modal>
                </div>
              ),
            },
            {
              key: 'record',
              label: '记录',
              children: (
                <div className="settings-history-tab">
                  <div
                    style={{
                      marginBottom: 16,
                      display: 'flex',
                      flexWrap: 'wrap',
                      gap: 8,
                      justifyContent: 'space-between',
                      alignItems: 'center',
                    }}
                  >
                    <Space>
                      <HistoryOutlined />
                      <span style={{ fontWeight: 600 }}>飞书历史消息</span>
                    </Space>
                    <Space wrap>
                      <Select
                        placeholder="筛选群聊"
                        allowClear
                        style={{ width: 200 }}
                        value={historySelectedChatId}
                        onChange={setHistorySelectedChatId}
                        onClear={() => setHistorySelectedChatId(undefined)}
                      >
                        {historyChats.map((chat) => (
                          <Select.Option key={chat.chat_id} value={chat.chat_id}>
                            {chat.chat_name || chat.chat_id}
                          </Select.Option>
                        ))}
                      </Select>
                      <Button icon={<ReloadOutlined />} onClick={loadHistoryMessages} size="middle">
                        刷新
                      </Button>
                      <Button type="primary" icon={<PlusOutlined />} onClick={() => setHistoryAddModalOpen(true)} size="middle">
                        添加
                      </Button>
                    </Space>
                  </div>

                  <Table
                    dataSource={historyMessages}
                    rowKey="id"
                    loading={historyLoading}
                    scroll={{ x: 'max-content' }}
                    pagination={{
                      current: historyPage,
                      pageSize: historyPageSize,
                      total: historyTotal,
                      showSizeChanger: true,
                      showQuickJumper: true,
                      showTotal: (t) => `共 ${t} 条`,
                      onChange: (p, ps) => {
                        setHistoryPage(p);
                        setHistoryPageSize(ps);
                      },
                    }}
                    size="middle"
                    columns={[
                      {
                        title: '时间',
                        dataIndex: 'created_at',
                        key: 'created_at',
                        width: 150,
                        render: (text: string) => {
                          if (!text) return '-';
                          const d = new Date(text);
                          return isNaN(d.getTime()) ? text : d.toLocaleString('zh-CN');
                        },
                      },
                      {
                        title: '发送者',
                        key: 'sender',
                        width: 120,
                        render: (_, record) => {
                          const isBot = record.sender_type === 'app';
                          return (
                            <Space>
                              <AntTag color={isBot ? 'blue' : 'green'}>
                                {isBot ? '智能体' : '用户'}
                              </AntTag>
                              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                                {record.sender_nickname || record.sender_open_id?.slice(0, 8) || '-'}
                              </Typography.Text>
                            </Space>
                          );
                        },
                      },
                      {
                        title: '内容',
                        dataIndex: 'content',
                        key: 'content',
                        ellipsis: true,
                        render: (content: string, record) => {
                          if (record.msg_type === 'text') {
                            try {
                              const parsed = JSON.parse(content);
                              return parsed.text || content;
                            } catch {
                              return content;
                            }
                          }
                          return <AntTag>{record.msg_type}</AntTag>;
                        },
                      },
                    ]}
                  />

                  <Modal
                    title="添加监听群聊"
                    open={historyAddModalOpen}
                    onOk={handleAddHistoryChat}
                    onCancel={() => {
                      setHistoryAddModalOpen(false);
                      historyForm.resetFields();
                    }}
                    width={520}
                  >
                    <Form form={historyForm} layout="vertical">
                      <Form.Item
                        name="bot_id"
                        label="机器人"
                        rules={[{ required: true, message: '请选择机器人' }]}
                      >
                        <Select placeholder="请选择机器人">
                          {agentBots.filter(b => b.bot_type === 'feishu').map((bot) => (
                            <Select.Option key={bot.id} value={bot.id}>
                              {bot.bot_name}
                            </Select.Option>
                          ))}
                        </Select>
                      </Form.Item>
                      <Form.Item
                        name="chat_id"
                        label="群聊 ID"
                        rules={[{ required: true, message: '请输入群聊 ID' }]}
                      >
                        <Input placeholder="请输入飞书群聊 ID" />
                      </Form.Item>
                      <Form.Item name="chat_name" label="群聊名称（可选）">
                        <Input placeholder="请输入群聊名称，方便识别" />
                      </Form.Item>
                    </Form>
                  </Modal>
                </div>
              ),
            },
          ]}
        />
      ),
    },
    {
      key: 'about',
      label: (
        <span>
          <InfoCircleOutlined style={{ marginRight: 6 }} />
          关于
        </span>
      ),
      children: (
        <Spin spinning={versionLoading}>
          <Card title="NTD 版本信息" style={{ maxWidth: 600 }}>
            {versionInfo ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
                <div>
                  <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginBottom: 4 }}>版本号</div>
                  <div style={{ fontSize: 24, fontWeight: 700, fontFamily: 'monospace' }}>{versionInfo.version}</div>
                </div>
                <div style={{ borderTop: '1px solid var(--color-border-light)', paddingTop: 16 }}>
                  <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginBottom: 8 }}>详细信息</div>
                  <Space direction="vertical" size={8}>
                    <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                      <span style={{ fontWeight: 500, minWidth: 80 }}>Git SHA:</span>
                      <code style={{ background: 'var(--color-bg-elevated)', padding: '2px 8px', borderRadius: 4, fontFamily: 'monospace' }}>{versionInfo.git_sha}</code>
                    </div>
                    <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                      <span style={{ fontWeight: 500, minWidth: 80 }}>Git Tag:</span>
                      <code style={{ background: 'var(--color-bg-elevated)', padding: '2px 8px', borderRadius: 4, fontFamily: 'monospace' }}>{versionInfo.git_describe}</code>
                    </div>
                  </Space>
                </div>
                <div style={{ borderTop: '1px solid var(--color-border-light)', paddingTop: 16 }}>
                  <Paragraph type="secondary" style={{ margin: 0 }}>
                    NTD (Nothing Todo) 是一个 AI Todo 应用，支持 Claude Code 和 JoinAI 等多种执行器。
                  </Paragraph>
                </div>
              </div>
            ) : (
              <Empty description="无法获取版本信息" />
            )}
          </Card>
        </Spin>
      ),
    },
  ];

  return (
    <div
      className="settings-page-root"
      style={{
        height: '100%',
        overflowY: 'auto',
        padding: '24px 32px',
        background: 'var(--color-bg-layout, #f8fafc)',
      }}
    >
      <div style={{ marginBottom: 24, display: 'flex', alignItems: 'center', gap: 12 }}>
        {onBack && (
          <button
            onClick={onBack}
            style={{
              background: 'var(--color-bg-elevated)',
              border: '1px solid var(--color-border)',
              borderRadius: 8,
              padding: '6px 12px',
              cursor: 'pointer',
              color: 'var(--color-text)',
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              flexShrink: 0,
            }}
          >
            ← 返回
          </button>
        )}
        <div style={{ minWidth: 0 }}>
          <h2 style={{ margin: 0, fontSize: 22, fontWeight: 700 }}>配置管理</h2>
          <Paragraph type="secondary" style={{ marginTop: 4, fontSize: 13 }}>
            管理系统配置、执行器路径、标签、备份和消息智能体
          </Paragraph>
        </div>
      </div>
      <Tabs items={tabItems} type="card" size="small" />

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

      <Modal
        title="选择性导出"
        open={exportModalOpen}
        onCancel={() => setExportModalOpen(false)}
        onOk={handleExportSelected}
        okText={`导出 ${exportTodoKeys.length} 项`}
        cancelText="取消"
        confirmLoading={exportingSelected}
        width={700}
        okButtonProps={{ disabled: exportTodoKeys.length === 0 }}
      >
        <Table
          dataSource={state.todos}
          rowKey="id"
          size="small"
          pagination={{ pageSize: 50 }}
          scroll={{ y: 400 }}
          rowSelection={{
            selectedRowKeys: exportTodoKeys,
            onChange: (keys) => setExportTodoKeys(keys as number[]),
          }}
          columns={[
            {
              title: '标题',
              dataIndex: 'title',
              ellipsis: true,
            },
            {
              title: '执行器',
              dataIndex: 'executor',
              width: 100,
              render: (v: string | undefined) => v || '-',
            },
            {
              title: '状态',
              dataIndex: 'status',
              width: 80,
              render: (v: string) => {
                const map: Record<string, { color: string; label: string }> = {
                  pending: { color: 'default', label: '待办' },
                  running: { color: 'processing', label: '进行中' },
                  completed: { color: 'success', label: '完成' },
                  failed: { color: 'error', label: '失败' },
                };
                const s = map[v] || { color: 'default', label: v };
                return <AntTag color={s.color}>{s.label}</AntTag>;
              },
            },
          ]}
        />
      </Modal>
    </div>
  );
}
