import { useState, useEffect } from 'react';
import { Modal, Select, Input, Button, Switch, Divider, Tag, App } from 'antd';
import { ClockCircleOutlined } from '@ant-design/icons';
import * as db from '../utils/database';
import type { Todo } from '../types';

const cronPresets = [
  { label: '每分钟', value: '0 * * * * *' },
  { label: '每5分钟', value: '0 */5 * * * *' },
  { label: '每10分钟', value: '0 */10 * * * *' },
  { label: '每30分钟', value: '0 */30 * * * *' },
  { label: '每小时', value: '0 0 * * * *' },
  { label: '每天早8点', value: '0 0 8 * * *' },
  { label: '每天午夜', value: '0 0 0 * * *' },
  { label: '每周一8点', value: '0 0 8 * * 1' },
  { label: '自定义', value: 'custom' },
];

interface TodoSettingsModalProps {
  open: boolean;
  todo: Todo | null;
  onClose: () => void;
  onUpdated: () => void;
}

export function TodoSettingsModal({ open, todo, onClose, onUpdated }: TodoSettingsModalProps) {
  const { message } = App.useApp();
  const [executor, setExecutor] = useState<string>('claudecode');
  const [schedulerEnabled, setSchedulerEnabled] = useState(false);
  const [schedulerConfig, setSchedulerConfig] = useState<string>('');
  const [cronPreset, setCronPreset] = useState<string>('custom');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (todo) {
      setExecutor(todo.executor || 'claudecode');
      setSchedulerEnabled(todo.scheduler_enabled || false);
      setSchedulerConfig(todo.scheduler_config || '');
      const matched = cronPresets.find((p) => p.value === todo.scheduler_config);
      setCronPreset(matched ? matched.value : 'custom');
    }
  }, [todo, open]);

  const handleCronPresetChange = (value: string) => {
    setCronPreset(value);
    if (value !== 'custom') {
      setSchedulerConfig(value);
    }
  };

  const handleSave = async () => {
    if (!todo) return;
    setLoading(true);
    try {
      // Update executor via updateTodo API
      await db.updateTodo(
        todo.id,
        todo.title,
        todo.description || '',
        todo.status,
        executor,
        schedulerEnabled,
        schedulerConfig || null,
      );

      // Update scheduler via dedicated API
      await db.updateScheduler(todo.id, schedulerEnabled, schedulerConfig || null);

      message.success('设置已保存');
      onUpdated();
      onClose();
    } catch (error) {
      message.error('保存失败: ' + error);
    } finally {
      setLoading(false);
    }
  };

  const executorLabel = executor === 'claudecode' ? 'Claude' : executor === 'opencode' ? 'Opencode' : 'JoinAI';
  const executorColor = executor === 'claudecode' ? '#7c3aed' : executor === 'opencode' ? '#f59e0b' : '#0d9488';

  return (
    <Modal
      title="任务设置"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="cancel" onClick={onClose}>取消</Button>,
        <Button key="save" type="primary" loading={loading} onClick={handleSave}>保存</Button>,
      ]}
    >
      {/* Executor */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 8, fontWeight: 600, fontSize: 14 }}>执行器</div>
        <Select
          value={executor}
          onChange={setExecutor}
          style={{ width: '100%' }}
          options={[
            { value: 'claudecode', label: 'Claude' },
            { value: 'joinai', label: 'JoinAI' },
            { value: 'opencode', label: 'Opencode' },
          ]}
        />
        <div style={{ marginTop: 8 }}>
          <Tag color={executorColor} style={{ fontWeight: 600 }}>
            {executorLabel}
          </Tag>
        </div>
      </div>

      <Divider style={{ margin: '16px 0' }} />

      {/* Scheduler */}
      <div>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 12,
          }}
        >
          <div style={{ fontWeight: 600, fontSize: 14 }}>
            <ClockCircleOutlined style={{ color: 'var(--color-primary)', marginRight: 6 }} />
            定时调度
          </div>
          <Switch
            checked={schedulerEnabled}
            onChange={(checked) => {
              setSchedulerEnabled(checked);
              if (checked && !schedulerConfig) {
                setSchedulerConfig('0 */10 * * * *');
                setCronPreset('0 */10 * * * *');
              }
            }}
          />
        </div>

        {schedulerEnabled && (
          <div style={{ marginTop: 12 }}>
            <Select
              value={cronPreset}
              onChange={handleCronPresetChange}
              style={{ width: '100%', marginBottom: 8 }}
              placeholder="选择预设或自定义"
              options={cronPresets}
            />
            {cronPreset === 'custom' && (
              <Input
                value={schedulerConfig}
                onChange={(e) => setSchedulerConfig(e.target.value)}
                placeholder="Cron 表达式，例如: 0 */10 * * * *"
                style={{ marginBottom: 8 }}
              />
            )}
          </div>
        )}

        {todo?.scheduler_config && (
          <div style={{ marginTop: 8, fontSize: 12, color: 'var(--color-text-tertiary)' }}>
            当前配置: <code>{todo.scheduler_config}</code>
          </div>
        )}
      </div>
    </Modal>
  );
}
