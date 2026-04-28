import { useState, useEffect } from 'react';
import { Drawer, Input, Button, Switch, Divider, App, Space, Tag } from 'antd';
import { ClockCircleOutlined, CheckOutlined } from '@ant-design/icons';
import * as db from '../utils/database';
import { EXECUTORS } from '../types';
import type { Todo } from '../types';
import { TagCheckCardGroup } from './TagCheckCard';
import parseExpression from 'cron-parser';

interface TodoSettingsDrawerProps {
  open: boolean;
  todo: Todo | null;
  tags: Array<{ id: number; name: string; color: string }>;
  onClose: () => void;
  onUpdated: () => void;
}

const CRON_PRESETS = [
  { label: '每10分钟', value: '0 */10 * * * *', category: '常用' },
  { label: '每30分钟', value: '0 */30 * * * *', category: '常用' },
  { label: '每1小时', value: '0 0 * * * *', category: '常用' },
  { label: '每2小时', value: '0 0 */2 * * *', category: '常用' },
  { label: '每6小时', value: '0 0 */6 * * *', category: '常用' },
  { label: '每天0点', value: '0 0 0 * * *', category: '定时' },
  { label: '每天9:00', value: '0 0 9 * * *', category: '定时' },
  { label: '每天18:00', value: '0 0 18 * * *', category: '定时' },
  { label: '工作日9-18点每小时', value: '0 0 9-18 * * 1-5', category: '工作时间' },
  { label: '工作日10:00', value: '0 0 10 * * 1-5', category: '工作时间' },
  { label: '工作日14:00', value: '0 0 14 * * 1-5', category: '工作时间' },
  { label: '22:00-08:00每45分钟', value: '0 */45 22-23,0-8 * * *', category: '下班时间' },
  { label: '22:00-08:00每小时', value: '0 0 22-23,0-8 * * *', category: '下班时间' },
];

export function TodoSettingsDrawer({ open, todo, tags, onClose, onUpdated }: TodoSettingsDrawerProps) {
  const { message } = App.useApp();
  const [executor, setExecutor] = useState<string>('claudecode');
  const [schedulerEnabled, setSchedulerEnabled] = useState(false);
  const [schedulerConfig, setSchedulerConfig] = useState<string>('');
  const [selectedTags, setSelectedTags] = useState<number[]>([]);

  const [cronSecond, setCronSecond] = useState<string>('');
  const [cronMinute, setCronMinute] = useState<string>('');
  const [cronHour, setCronHour] = useState<string>('');
  const [cronDay, setCronDay] = useState<string>('');
  const [cronMonth, setCronMonth] = useState<string>('');
  const [cronWeekday, setCronWeekday] = useState<string>('');

  const [loading, setLoading] = useState(false);

  const setCronFields = (config: string) => {
    const parts = config.split(' ');
    if (parts.length >= 6) {
      setCronSecond(parts[0]);
      setCronMinute(parts[1]);
      setCronHour(parts[2]);
      setCronDay(parts[3]);
      setCronMonth(parts[4]);
      setCronWeekday(parts[5]);
    }
  };

  const handlePresetSelect = (presetValue: string) => {
    setSchedulerConfig(presetValue);
    setCronFields(presetValue);
  };

  const validateCronExpression = (expr: string): { valid: boolean; error?: string } => {
    if (!expr) return { valid: false, error: 'Cron 表达式不能为空' };
    const parts = expr.split(' ');
    if (parts.length !== 6) return { valid: false, error: 'Cron 表达式必须包含6个字段（秒 分 时 日 月 星期）' };
    try {
      parseExpression.parse(expr);
      return { valid: true };
    } catch (error: any) {
      return { valid: false, error: `无效的 Cron 表达式: ${error.message}` };
    }
  };

  useEffect(() => {
    if (todo) {
      setExecutor(todo.executor || 'claudecode');
      setSchedulerEnabled(todo.scheduler_enabled || false);
      setSchedulerConfig(todo.scheduler_config || '');
      setSelectedTags((todo as any).tag_ids || []);
      if (todo.scheduler_config) {
        setCronFields(todo.scheduler_config);
      } else {
        setCronFields('* * * * * *');
      }
    }
  }, [todo, open]);

  const handleSave = async () => {
    if (!todo) return;

    if (schedulerEnabled) {
      const validation = validateCronExpression(schedulerConfig);
      if (!validation.valid) {
        message.error(validation.error);
        return;
      }
    }

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
                const defaultCron = '0 */10 * * * *';
                setSchedulerConfig(defaultCron);
                setCronFields(defaultCron);
              }
            }}
          />
        </div>

        {schedulerEnabled && (
          <div style={{ marginTop: 12 }}>
            <div style={{ marginBottom: 8 }}>
              <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginBottom: 6, fontWeight: 500 }}>
                快捷选择
              </div>
              <Space direction="vertical" style={{ width: '100%' }} size={8}>
                {['常用', '定时', '工作时间', '下班时间'].map(category => {
                  const presets = CRON_PRESETS.filter(p => p.category === category);
                  return (
                    <div key={category}>
                      <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginBottom: 4 }}>
                        {category}
                      </div>
                      <Space wrap size={6}>
                        {presets.map(preset => (
                          <Tag
                            key={preset.value}
                            color={schedulerConfig === preset.value ? 'blue' : 'default'}
                            style={{
                              cursor: 'pointer',
                              padding: '4px 10px',
                              fontSize: 12,
                              borderRadius: 4,
                              border: schedulerConfig === preset.value ? '2px solid #1677ff' : '1px solid var(--color-border)',
                              fontWeight: schedulerConfig === preset.value ? 600 : 400,
                            }}
                            onClick={() => handlePresetSelect(preset.value)}
                          >
                            {preset.label}
                          </Tag>
                        ))}
                      </Space>
                    </div>
                  );
                })}
              </Space>
            </div>

            <div>
              <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginBottom: 6, fontWeight: 500 }}>
                Cron 表达式配置
              </div>
              <div style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(80px, 1fr))',
                gap: '8px 12px',
                background: 'var(--color-fill-quaternary)',
                padding: '12px',
                borderRadius: '8px',
                border: '1px solid var(--color-border-secondary)',
              }}>
                {[
                  { label: '秒', value: cronSecond, onChange: setCronSecond, placeholder: '0-59' },
                  { label: '分', value: cronMinute, onChange: setCronMinute, placeholder: '0-59' },
                  { label: '时', value: cronHour, onChange: setCronHour, placeholder: '0-23' },
                  { label: '日', value: cronDay, onChange: setCronDay, placeholder: '1-31' },
                  { label: '月', value: cronMonth, onChange: setCronMonth, placeholder: '1-12' },
                  { label: '星期', value: cronWeekday, onChange: setCronWeekday, placeholder: '0-6' },
                ].map((field, index) => (
                  <div key={field.label}>
                    <div style={{
                      fontSize: 11,
                      color: 'var(--color-text-tertiary)',
                      marginBottom: 4,
                      textAlign: 'center',
                      fontWeight: 500,
                    }}>
                      {field.label}
                    </div>
                    <Input
                      value={field.value}
                      onChange={(e) => {
                        const newVal = e.target.value;
                        field.onChange(newVal);
                        const parts = [cronSecond, cronMinute, cronHour, cronDay, cronMonth, cronWeekday];
                        parts[index] = newVal;
                        setSchedulerConfig(parts.join(' '));
                      }}
                      placeholder={field.placeholder}
                      style={{
                        fontSize: 12,
                        textAlign: 'center',
                        fontFamily: 'monospace',
                        borderColor: index < 5 ? undefined : '#1677ff',
                      }}
                    />
                  </div>
                ))}
              </div>
              <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginTop: 6 }}>
                提示: * (任意) */n (每n) n-m (范围) n,m,n (多个值，如 1,3,5 表示第1、3、5)
              </div>
            </div>
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
