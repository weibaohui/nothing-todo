import { useState, useEffect } from 'react';
import { Modal, Input, Button, Switch, Divider, App } from 'antd';
import { ClockCircleOutlined, CheckOutlined } from '@ant-design/icons';
import * as db from '../utils/database';
import { EXECUTORS } from '../types';
import type { Todo } from '../types';

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
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (todo) {
      setExecutor(todo.executor || 'claudecode');
      setSchedulerEnabled(todo.scheduler_enabled || false);
      setSchedulerConfig(todo.scheduler_config || '');
    }
  }, [todo, open]);

  const handleSave = async () => {
    if (!todo) return;
    setLoading(true);
    try {
      await db.updateTodo(
        todo.id,
        todo.title,
        todo.prompt || '',
        todo.status,
        executor,
        schedulerEnabled,
        schedulerConfig || null,
      );

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
      {/* Executor Selection */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 10, fontWeight: 600, fontSize: 14 }}>执行器</div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 10 }}>
          {EXECUTORS.map((opt) => {
            const selected = executor === opt.value;
            return (
              <div
                key={opt.value}
                onClick={() => setExecutor(opt.value)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    setExecutor(opt.value);
                  }
                }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '10px 14px',
                  borderRadius: 10,
                  border: `2px solid ${selected ? opt.color : '#e2e8f0'}`,
                  background: selected ? `${opt.color}10` : '#fff',
                  cursor: 'pointer',
                  transition: 'all 0.2s ease',
                  flex: '1 1 calc(50% - 10px)',
                  minWidth: 120,
                }}
                onMouseEnter={(e) => {
                  if (!selected) {
                    (e.currentTarget as HTMLDivElement).style.borderColor = `${opt.color}60`;
                    (e.currentTarget as HTMLDivElement).style.background = `${opt.color}08`;
                  }
                }}
                onMouseLeave={(e) => {
                  if (!selected) {
                    (e.currentTarget as HTMLDivElement).style.borderColor = '#e2e8f0';
                    (e.currentTarget as HTMLDivElement).style.background = '#fff';
                  }
                }}
              >
                <span style={{ fontSize: 16, lineHeight: 1 }}>{opt.icon}</span>
                <span style={{
                  fontSize: 14,
                  fontWeight: 600,
                  color: selected ? opt.color : '#0f172a',
                  flex: 1,
                }}>
                  {opt.label}
                </span>
                {selected && (
                  <span style={{
                    width: 18,
                    height: 18,
                    borderRadius: '50%',
                    backgroundColor: opt.color,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                  }}>
                    <CheckOutlined style={{ fontSize: 10, color: '#fff' }} />
                  </span>
                )}
              </div>
            );
          })}
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
              }
            }}
          />
        </div>

        {schedulerEnabled && (
          <div style={{ marginTop: 12 }}>
            <Input
              value={schedulerConfig}
              onChange={(e) => setSchedulerConfig(e.target.value)}
              placeholder="Cron 表达式，例如: 0 */10 * * * *"
              style={{ marginBottom: 8 }}
            />
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
