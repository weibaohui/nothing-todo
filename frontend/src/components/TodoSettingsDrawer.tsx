import { useState, useEffect } from 'react';
import { Drawer, Input, Button, Switch, Divider, App } from 'antd';
import { ClockCircleOutlined, CheckOutlined, FolderOutlined } from '@ant-design/icons';
import { Cron } from 'react-js-cron';
import 'react-js-cron/dist/styles.css';
import * as db from '../utils/database';
import { CRON_ZH_LOCALE, cronTo5, cronTo6 } from '../utils/cron';
import { EXECUTORS, executorConfigToOption } from '../types';
import type { Todo, ExecutorConfig, ExecutorOption } from '../types';
import { TagCheckCardGroup } from './TagCheckCard';
import { CronPresetSelect } from './CronPresetSelect';

interface TodoSettingsDrawerProps {
  open: boolean;
  todo: Todo | null;
  tags: Array<{ id: number; name: string; color: string }>;
  onClose: () => void;
  onUpdated: () => void;
}

const DEFAULT_CRON = '0 */10 * * * *';

export function TodoSettingsDrawer({ open, todo, tags, onClose, onUpdated }: TodoSettingsDrawerProps) {
  const { message } = App.useApp();
  const [executor, setExecutor] = useState<string>('claudecode');
  const [schedulerEnabled, setSchedulerEnabled] = useState(false);
  const [schedulerConfig, setSchedulerConfig] = useState<string>('');
  const [selectedTags, setSelectedTags] = useState<number[]>([]);
  const [workspace, setWorkspace] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [executorOptions, setExecutorOptions] = useState<ExecutorOption[]>(EXECUTORS);

  useEffect(() => {
    db.getExecutors()
      .then((list: ExecutorConfig[]) => {
        const enabled = list.filter((ec) => ec.enabled);
        if (enabled.length > 0) {
          setExecutorOptions(enabled.map(executorConfigToOption));
        }
      })
      .catch(() => {});
  }, [open]);

  useEffect(() => {
    if (todo) {
      setExecutor(todo.executor || 'claudecode');
      setSchedulerEnabled(todo.scheduler_enabled || false);
      setSchedulerConfig(todo.scheduler_config || '');
      setSelectedTags((todo as any).tag_ids || []);
      setWorkspace(todo.workspace || '');
    }
  }, [todo, open]);

  const handleSave = async () => {
    if (!todo) return;

    setLoading(true);
    try {
      const trimmedWorkspace = workspace.trim() || null;
      await db.updateTodo(
        todo.id,
        todo.title,
        todo.prompt || '',
        todo.status,
        executor,
        schedulerEnabled,
        schedulerConfig || null,
        trimmedWorkspace,
      );
      await db.updateScheduler(todo.id, schedulerEnabled, schedulerConfig || null);
      await db.updateTodoTags(todo.id, selectedTags);

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
    <Drawer
      title="任务设置"
      open={open}
      onClose={onClose}
      width={420}
      placement="right"
      extra={
        <Button type="primary" loading={loading} onClick={handleSave}>
          保存
        </Button>
      }
    >
      {/* Tags */}
      {tags.length > 0 && (
        <>
          <div style={{ marginBottom: 16 }}>
            <div style={{ marginBottom: 10, fontWeight: 600, fontSize: 14 }}>标签</div>
            <TagCheckCardGroup
              tags={tags}
              value={selectedTags[0] || null}
              onChange={(val) => setSelectedTags(val ? [val as number] : [])}
            />
          </div>
          <Divider style={{ margin: '8px 0 16px' }} />
        </>
      )}

      {/* Executor Selection */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 10, fontWeight: 600, fontSize: 14 }}>执行器</div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 10 }}>
          {executorOptions.map((opt) => {
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
                  border: `2px solid ${selected ? opt.color : 'var(--color-border-secondary)'}`,
                  background: selected ? `${opt.color}10` : 'var(--color-bg-elevated)',
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
                    (e.currentTarget as HTMLDivElement).style.borderColor = 'var(--color-border-secondary)';
                    (e.currentTarget as HTMLDivElement).style.background = 'var(--color-bg-elevated)';
                  }
                }}
              >
                <span style={{ fontSize: 16, lineHeight: 1 }}>{opt.icon}</span>
                <span style={{
                  fontSize: 14,
                  fontWeight: 600,
                  color: selected ? opt.color : 'var(--color-text)',
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

      {/* Workspace */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 10, fontWeight: 600, fontSize: 14 }}>
          <FolderOutlined style={{ color: 'var(--color-primary)', marginRight: 6 }} />
          工作目录
        </div>
        <Input
          value={workspace}
          onChange={(e) => setWorkspace(e.target.value)}
          placeholder="设置执行器的工作目录（可选）"
          style={{ width: '100%' }}
        />
      </div>

      <Divider style={{ margin: '8px 0 16px' }} />

      {/* Scheduler */}
      <div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
          <div style={{ fontWeight: 600, fontSize: 14 }}>
            <ClockCircleOutlined style={{ color: 'var(--color-primary)', marginRight: 6 }} />
            定时调度
          </div>
          <Switch
            checked={schedulerEnabled}
            onChange={(checked) => {
              setSchedulerEnabled(checked);
              if (checked && !schedulerConfig) {
                setSchedulerConfig(DEFAULT_CRON);
              }
            }}
          />
        </div>

        {schedulerEnabled && (
          <div style={{ marginTop: 12 }}>
            <CronPresetSelect
              value={schedulerConfig || DEFAULT_CRON}
              onChange={(val) => setSchedulerConfig(val)}
            />
            <Cron
              value={cronTo5(schedulerConfig || DEFAULT_CRON)}
              setValue={(val: string) => setSchedulerConfig(cronTo6(val))}
              locale={CRON_ZH_LOCALE}
              defaultPeriod="hour"
              humanizeLabels
              allowClear={false}
            />
          </div>
        )}

        {todo?.scheduler_config && (
          <div style={{ marginTop: 8, fontSize: 12, color: 'var(--color-text-tertiary)' }}>
            当前配置: <code>{todo.scheduler_config}</code>
          </div>
        )}
      </div>
    </Drawer>
  );
}
