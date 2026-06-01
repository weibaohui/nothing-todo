import { useState, useEffect, useMemo } from 'react';
import {
  Table, Button, Space, Modal, Form, Input, Select, Switch, message,
  Popconfirm, Tabs, Tag, Card, Row, Col, InputNumber, Typography, Divider,
} from 'antd';
import {
  PlusOutlined, DeleteOutlined, EditOutlined, PlayCircleOutlined,
  ReloadOutlined, ClearOutlined, CheckCircleOutlined, CloseCircleOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import * as db from '../../utils/database';
import type {
  HookRule, CreateHookRequest, UpdateHookRequest, GlobalHookConfig,
  HookLogEntry, HookFilter, HookAction,
} from '../../utils/database/hooks';
import { HOOK_TRIGGERS } from '../../utils/database/hooks';
import { useApp } from '../../hooks/useApp';
import { LinkOutlined } from '@ant-design/icons';

const { Text } = Typography;

function HookFilterEditor({ value, onChange }: { value?: HookFilter; onChange?: (v: HookFilter) => void }) {
  const [form] = Form.useForm();
  useEffect(() => {
    form.setFieldsValue(value || { status: [], title_contains: undefined, tags: [], executor: undefined });
  }, [value, form]);

  return (
    <Form form={form} layout="vertical" onValuesChange={(_, all) => onChange?.(all as HookFilter)}>
      <Row gutter={16}>
        <Col span={12}>
          <Form.Item name="status" label="状态过滤">
            <Select mode="multiple" placeholder="任意状态" allowClear>
              <Select.Option value="pending">待处理</Select.Option>
              <Select.Option value="in_progress">进行中</Select.Option>
              <Select.Option value="completed">已完成</Select.Option>
              <Select.Option value="failed">失败</Select.Option>
            </Select>
          </Form.Item>
        </Col>
        <Col span={12}>
          <Form.Item name="title_contains" label="标题包含">
            <Input placeholder="不区分大小写" />
          </Form.Item>
        </Col>
      </Row>
      <Row gutter={16}>
        <Col span={12}>
          <Form.Item name="executor" label="执行人">
            <Input placeholder="例如 claude" />
          </Form.Item>
        </Col>
        <Col span={12}>
          <Form.Item name="tags" label="标签 ID">
            <Select mode="tags" placeholder="标签 ID" allowClear>
            </Select>
          </Form.Item>
        </Col>
      </Row>
    </Form>
  );
}

function HookActionEditor({ value, onChange }: { value?: HookAction; onChange?: (v: HookAction) => void }) {
  const [form] = Form.useForm();
  const { state } = useApp();

  useEffect(() => {
    form.setFieldsValue(
      value || { target_todo_id: undefined, prompt_template: '', skip_if_missing: false }
    );
  }, [value, form]);

  const todoOptions = useMemo(
    () =>
      state.todos
        .slice()
        .sort((a, b) => a.title.localeCompare(b.title))
        .map((t) => ({
          label: `${t.title} (#${t.id})`,
          value: t.id,
        })),
    [state.todos]
  );

  return (
    <Form form={form} layout="vertical" onValuesChange={(_, all) => onChange?.(all as HookAction)}>
      <Form.Item
        name="target_todo_id"
        label="目标 Todo"
        tooltip="触发时要执行的 Todo 项（类似消息的默认响应）"
        rules={[{ required: true, message: '请选择要触发的 Todo' }]}
      >
        <Select
          showSearch
          placeholder="选择一个 Todo，触发时会执行它"
          optionFilterProp="label"
          options={todoOptions}
          notFoundContent={state.todos.length === 0 ? '暂无 Todo，请先创建' : '无匹配项'}
        />
      </Form.Item>
      <Form.Item
        name="prompt_template"
        label="Prompt 模板（可选）"
        tooltip="留空则使用目标 Todo 自身的 prompt。支持占位符：{{source_todo_id}} {{source_todo_title}} {{todo_id}} {{todo_title}} {{old_status}} {{new_status}} {{executor}} {{trigger}}"
      >
        <Input.TextArea
          rows={3}
          placeholder="例如：请基于 {{source_todo_title}} 给出分析"
        />
      </Form.Item>
      <Form.Item
        name="skip_if_missing"
        label="目标不存在时跳过"
        tooltip="开启后，若目标 Todo 已被删除，则仅记录警告而不让 hook 失败"
        valuePropName="checked"
      >
        <Switch />
      </Form.Item>
    </Form>
  );
}

interface HookFormData {
  name: string;
  description?: string;
  enabled: boolean;
  trigger: string;
  filter?: HookFilter;
  action: HookAction;
  is_async: boolean;
}

function HookModal({
  open, hook, onClose, onSave,
}: {
  open: boolean;
  hook?: HookRule;
  onClose: () => void;
  onSave: (data: CreateHookRequest | UpdateHookRequest) => Promise<void>;
}) {
  const [form] = Form.useForm<HookFormData>();
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      if (hook) {
        form.setFieldsValue({
          name: hook.name,
          description: hook.description || undefined,
          enabled: hook.enabled,
          trigger: hook.trigger,
          filter: hook.filter,
          action: hook.action,
          is_async: hook.is_async,
        });
      } else {
        form.setFieldsValue({
          name: '',
          description: '',
          enabled: true,
          trigger: 'before_create',
          filter: { status: [], title_contains: '', tags: [], executor: '' },
          action: { target_todo_id: undefined, prompt_template: '', skip_if_missing: false },
          is_async: true,
        });
      }
    }
  }, [open, hook, form]);

  const handleOk = async () => {
    try {
      const values = await form.validateFields();
      setSaving(true);
      await onSave(values);
      onClose();
    } catch {
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title={hook ? '编辑 Hook' : '创建 Hook'}
      open={open}
      onCancel={onClose}
      onOk={handleOk}
      width={700}
      confirmLoading={saving}
      okText="保存"
      cancelText="取消"
    >
      <Form form={form} layout="vertical">
        <Row gutter={16}>
          <Col span={16}>
            <Form.Item name="name" label="名称" rules={[{ required: true }]}>
              <Input placeholder="例如：创建时通知" />
            </Form.Item>
          </Col>
          <Col span={4}>
            <Form.Item name="enabled" label="启用" valuePropName="checked">
              <Switch />
            </Form.Item>
          </Col>
          <Col span={4}>
            <Form.Item name="is_async" label="异步" valuePropName="checked">
              <Switch />
            </Form.Item>
          </Col>
        </Row>
        <Form.Item name="description" label="描述">
          <Input placeholder="可选描述" />
        </Form.Item>
        <Form.Item name="trigger" label="触发器" rules={[{ required: true }]}>
          <Select>
            {HOOK_TRIGGERS.map(t => (
              <Select.Option key={t.value} value={t.value}>{t.label}</Select.Option>
            ))}
          </Select>
        </Form.Item>
        <Divider>过滤条件</Divider>
        <HookFilterEditor />
        <Divider>执行动作</Divider>
        <HookActionEditor />
      </Form>
    </Modal>
  );
}

function HookListTab() {
  const { state } = useApp();
  const [hooks, setHooks] = useState<HookRule[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingHook, setEditingHook] = useState<HookRule | undefined>();
  const [testingId, setTestingId] = useState<number | null>(null);
  const [testingResult, setTestingResult] = useState<string | null>(null);

  const loadHooks = async () => {
    setLoading(true);
    try {
      const data = await db.getHooks();
      setHooks(data);
    } catch (e: any) {
      message.error('加载 hooks 失败: ' + e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadHooks(); }, []);

  const handleSave = async (data: CreateHookRequest | UpdateHookRequest) => {
    if (editingHook) {
      await db.updateHook(editingHook.id, data);
      message.success('Hook updated');
    } else {
      await db.createHook(data as CreateHookRequest);
      message.success('Hook created');
    }
    loadHooks();
  };

  const handleDelete = async (id: number) => {
    await db.deleteHook(id);
    message.success('Hook deleted');
    loadHooks();
  };

  const handleTest = async (hook: HookRule) => {
    setTestingId(hook.id);
    setTestingResult(null);
    try {
      const result = await db.testHook(hook.id);
      const success = result.success ? 'SUCCESS' : 'FAILED';
      const output = `Exit Code: ${result.exit_code}\nDuration: ${result.duration_ms}ms\n\nStdout:\n${result.stdout || '(empty)'}\n\nStderr:\n${result.stderr || '(empty)'}`;
      setTestingResult(`[${success}] ${output}`);
    } catch (e: any) {
      setTestingResult('Error: ' + e.message);
    } finally {
      setTestingId(null);
    }
  };

  const renderTargetTodo = (action: HookAction) => {
    const todoId = action?.target_todo_id;
    if (todoId == null || Number.isNaN(todoId) || todoId < 0) {
      return <Text type="secondary">未配置</Text>;
    }
    const todo = state.todos.find((t) => t.id === todoId);
    if (!todo) {
      return (
        <Space size={4} direction="vertical" style={{ lineHeight: 1.2 }}>
          <Text type="warning">#{todoId}（已删除）</Text>
          {action.prompt_template && (
            <Text type="secondary" style={{ fontSize: 11 }} ellipsis>
              模板：{action.prompt_template}
            </Text>
          )}
        </Space>
      );
    }
    return (
      <Space size={4} direction="vertical" style={{ lineHeight: 1.2 }}>
        <Text>{todo.title}</Text>
        <Text type="secondary" style={{ fontSize: 11 }}>#{todo.id}</Text>
        {action.prompt_template && (
          <Text type="secondary" style={{ fontSize: 11 }} ellipsis>
            模板：{action.prompt_template}
          </Text>
        )}
      </Space>
    );
  };

  const columns: ColumnsType<HookRule> = [
    { title: '名称', dataIndex: 'name', key: 'name' },
    {
      title: '触发器', dataIndex: 'trigger', key: 'trigger',
      render: (t: string) => HOOK_TRIGGERS.find(x => x.value === t)?.label || t,
    },
    {
      title: '启用', dataIndex: 'enabled', key: 'enabled',
      render: (v: boolean) => v ? <CheckCircleOutlined style={{ color: 'green' }} /> : <CloseCircleOutlined style={{ color: 'red' }} />,
    },
    {
      title: '异步', dataIndex: 'is_async', key: 'is_async',
      render: (v: boolean) => v ? <Tag>异步</Tag> : <Tag color="orange">同步</Tag>,
    },
    {
      title: '目标 Todo', dataIndex: 'action', key: 'target_todo',
      render: (_: unknown, record) => renderTargetTodo(record.action),
    },
    {
      title: '操作', key: 'action', width: 200,
      render: (_, record) => (
        <Space>
          <Button size="small" icon={<PlayCircleOutlined />} loading={testingId === record.id}
            onClick={() => handleTest(record)}>测试</Button>
          <Button size="small" icon={<EditOutlined />} onClick={() => { setEditingHook(record); setModalOpen(true); }} />
          <Popconfirm title="确认删除？" onConfirm={() => handleDelete(record.id)}>
            <Button size="small" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => { setEditingHook(undefined); setModalOpen(true); }}>
          创建 Hook
        </Button>
        <Button icon={<ReloadOutlined />} onClick={loadHooks} style={{ marginLeft: 8 }}>刷新</Button>
      </div>
      <Table columns={columns} dataSource={hooks} rowKey="id" loading={loading} size="small" />
      {testingResult && (
        <Card title="测试结果" size="small" style={{ marginTop: 16 }}>
          <pre style={{ maxHeight: 300, overflow: 'auto', fontSize: 12 }}>{testingResult}</pre>
        </Card>
      )}
      <HookModal
        open={modalOpen}
        hook={editingHook}
        onClose={() => { setModalOpen(false); setEditingHook(undefined); }}
        onSave={handleSave}
      />
    </div>
  );
}

function GlobalConfigTab() {
  const [config, setConfig] = useState<GlobalHookConfig | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => { loadConfig(); }, []);

  const loadConfig = async () => {
    try {
      const data = await db.getGlobalHookConfig();
      setConfig(data);
    } catch (e: any) {
      message.error('加载配置失败: ' + e.message);
    }
  };

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    try {
      await db.updateGlobalHookConfig(config);
      message.success('配置已保存');
    } catch (e: any) {
      message.error('保存失败: ' + e.message);
    } finally {
      setSaving(false);
    }
  };

  if (!config) return null;

  return (
    <Card>
      <Row gutter={16}>
        <Col span={8}>
          <Form.Item label="启用">
            <Switch checked={config.enabled} onChange={(v) => setConfig({ ...config, enabled: v })} />
          </Form.Item>
        </Col>
        <Col span={8}>
          <Form.Item label="默认超时秒数">
            <InputNumber
              value={config.default_timeout_secs}
              onChange={(v) => setConfig({ ...config, default_timeout_secs: v || 30 })}
              min={1} max={3600}
            />
          </Form.Item>
        </Col>
        <Col span={8}>
          <Form.Item label="最大并发数">
            <InputNumber
              value={config.max_concurrency}
              onChange={(v) => setConfig({ ...config, max_concurrency: v || 5 })}
              min={1} max={50}
            />
          </Form.Item>
        </Col>
      </Row>
      <Button type="primary" onClick={handleSave} loading={saving}>保存配置</Button>
    </Card>
  );
}

function LogsTab({ onBack }: { onBack?: () => void }) {
  const { state, dispatch } = useApp();
  const [logs, setLogs] = useState<HookLogEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(1);
  const [status, setStatus] = useState<string | undefined>();

  const loadLogs = async () => {
    setLoading(true);
    try {
      const data = await db.getHookLogs({ page, limit: 20, status });
      setLogs(data.logs);
      setTotal(data.total);
    } catch (e: any) {
      message.error('加载日志失败: ' + e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadLogs(); }, [page, status]);

  const handleClear = async () => {
    try {
      await db.clearHookLogs();
      message.success('Logs cleared');
      loadLogs();
    } catch (e: any) {
      message.error('清空失败: ' + e.message);
    }
  };

  const handleOpenTodo = (todoId: number) => {
    if (!state.todos.some(t => t.id === todoId)) {
      message.warning('该 Todo 已被删除或不可访问');
      return;
    }
    dispatch({ type: 'SELECT_TODO', payload: todoId });
    onBack?.();
  };

  const renderTodoCell = (todoId: number | null) => {
    if (todoId == null) {
      return <Text type="secondary" style={{ fontSize: 12 }}>（新建时触发）</Text>;
    }
    const todo = state.todos.find(t => t.id === todoId);
    const title = todo?.title || `Todo #${todoId}`;
    const exists = !!todo;
    if (!exists) {
      return (
        <Space size={4}>
          <Text type="secondary" style={{ fontSize: 12 }}>#{todoId}（已删除）</Text>
        </Space>
      );
    }
    return (
      <Button
        type="link"
        size="small"
        icon={<LinkOutlined />}
        onClick={() => handleOpenTodo(todoId)}
        style={{ padding: 0, height: 'auto', textAlign: 'left', whiteSpace: 'normal' }}
        title={`跳转到 Todo #${todoId}`}
      >
        <Space direction="vertical" size={0} style={{ lineHeight: 1.3 }}>
          <span>{title}</span>
          <Text type="secondary" style={{ fontSize: 11 }}>#{todoId}</Text>
        </Space>
      </Button>
    );
  };

  const columns: ColumnsType<HookLogEntry> = [
    {
      title: '时间', dataIndex: 'created_at', key: 'created_at',
      render: (t: string) => new Date(t).toLocaleString(),
      width: 160,
    },
    { title: 'Hook 名称', dataIndex: 'hook_name', key: 'hook_name' },
    { title: '触发器', dataIndex: 'trigger', key: 'trigger' },
    {
      title: '所属 Todo', dataIndex: 'todo_id', key: 'todo_id',
      width: 220,
      render: renderTodoCell,
    },
    {
      title: '状态', dataIndex: 'success', key: 'success',
      render: (v: boolean | null) => v ? <Tag color="green">成功</Tag> : <Tag color="red">失败</Tag>,
    },
    {
      title: '耗时', dataIndex: 'duration_ms', key: 'duration_ms',
      render: (v: number | null) => v ? `${v}ms` : '-',
    },
    {
      title: '退出码', dataIndex: 'exit_code', key: 'exit_code',
      render: (v: number | null) => v ?? '-',
    },
    { title: '错误', dataIndex: 'error_msg', key: 'error_msg', ellipsis: true },
  ];

  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <Select placeholder="按状态筛选" allowClear style={{ width: 150, marginRight: 8 }}
          onChange={(v) => { setStatus(v); setPage(1); }}>
          <Select.Option value="success">成功</Select.Option>
          <Select.Option value="failed">失败</Select.Option>
        </Select>
        <Button icon={<ReloadOutlined />} onClick={loadLogs}>刷新</Button>
        <Popconfirm title="确认清空所有日志？" onConfirm={handleClear}>
          <Button danger icon={<ClearOutlined />} style={{ marginLeft: 8 }}>清空全部</Button>
        </Popconfirm>
        <Text type="secondary" style={{ marginLeft: 16 }}>总计：{total}</Text>
      </div>
      <Table columns={columns} dataSource={logs} rowKey="id" loading={loading} size="small"
        pagination={{ current: page, pageSize: 20, total, onChange: setPage }} />
    </div>
  );
}

export function HooksPanel({ onBack }: { onBack?: () => void }) {
  const tabItems = [
    { key: 'hooks', label: 'Hook 规则', children: <HookListTab /> },
    { key: 'config', label: '全局配置', children: <GlobalConfigTab /> },
    { key: 'logs', label: '执行日志', children: <LogsTab onBack={onBack} /> },
  ];

  return <Tabs items={tabItems} />;
}
