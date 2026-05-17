import { useState, useEffect } from 'react';
import { Drawer, Input, Button, App, AutoComplete, Divider, Switch } from 'antd';
import { CheckOutlined, FolderOutlined, ClockCircleOutlined, FileTextOutlined } from '@ant-design/icons';
import { Cron } from 'react-js-cron';
import 'react-js-cron/dist/styles.css';
import * as db from '../utils/database';
import type { ProjectDirectory } from '../utils/database';
import type { Todo, ExecutorConfig, ExecutorOption } from '../types';
import { CRON_ZH_LOCALE, cronTo5, cronTo6 } from '../utils/cron';
import { EXECUTORS, executorConfigToOption } from '../types';
import { TagCheckCardGroup } from './TagCheckCard';
import { CronPresetSelect } from './CronPresetSelect';
import { MdEditor } from './MdEditor';

interface TodoDrawerProps {
  open: boolean;
  todo: Todo | null; // null = create mode, Todo = edit mode
  tags: Array<{ id: number; name: string; color: string }>;
  onClose: () => void;
  onSaved: (todo?: Todo) => void; // callback after save
}

const DEFAULT_CRON = '0 */10 * * * *';

export function TodoDrawer({ open, todo, tags, onClose, onSaved }: TodoDrawerProps) {
  const { message } = App.useApp();
  const isEditMode = todo !== null;

  // Basic info
  const [title, setTitle] = useState('');
  const [prompt, setPrompt] = useState('');
  const [selectedTags, setSelectedTags] = useState<number[]>([]);

  // Executor & workspace
  const [executor, setExecutor] = useState<string>('claudecode');
  const [workspace, setWorkspace] = useState<string>('');
  const [worktreeEnabled, setWorktreeEnabled] = useState(false);
  const [executorOptions, setExecutorOptions] = useState<ExecutorOption[]>(EXECUTORS);
  const [projectDirectories, setProjectDirectories] = useState<ProjectDirectory[]>([]);

  // Scheduler
  const [schedulerEnabled, setSchedulerEnabled] = useState(false);
  const [schedulerConfig, setSchedulerConfig] = useState<string>('');

  // Loading states
  const [loading, setLoading] = useState(false);

  // Initialize data when drawer opens
  useEffect(() => {
    if (open) {
      Promise.all([
        db.getExecutors(),
        db.getProjectDirectories(),
      ]).then(([executorConfigs, directories]) => {
        const enabled = (executorConfigs as ExecutorConfig[]).filter((ec) => ec.enabled);
        if (enabled.length > 0) {
          setExecutorOptions(enabled.map(executorConfigToOption));
        }
        setProjectDirectories(directories);
      }).catch(() => {});
    }
  }, [open]);

  // Reset or populate form when todo changes
  useEffect(() => {
    if (open) {
      if (todo) {
        setTitle(todo.title || '');
        setPrompt(todo.prompt || '');
        setSelectedTags((todo as any).tag_ids || []);
        setExecutor(todo.executor || 'claudecode');
        setWorkspace(todo.workspace || '');
        setWorktreeEnabled(todo.worktree_enabled || false);
        setSchedulerEnabled(todo.scheduler_enabled || false);
        setSchedulerConfig(todo.scheduler_config || '');
      } else {
        // Create mode - reset
        setTitle('');
        setPrompt('');
        setSelectedTags([]);
        setExecutor('claudecode');
        setWorkspace('');
        setWorktreeEnabled(false);
        setSchedulerEnabled(false);
        setSchedulerConfig('');
      }
    }
  }, [open, todo]);

  const handleSave = async () => {
    if (!title.trim()) {
      message.error('请输入任务标题');
      return;
    }

    setLoading(true);
    try {
      const trimmedWorkspace = workspace.trim() || null;

      if (isEditMode && todo) {
        // Update existing todo
        if (trimmedWorkspace) {
          const exists = projectDirectories.some(d => d.path === trimmedWorkspace);
          if (!exists) {
            try {
              await db.createProjectDirectory(trimmedWorkspace);
            } catch {
              // Ignore
            }
          }
        }

        await db.updateTodo(
          todo.id,
          title.trim(),
          prompt.trim(),
          todo.status,
          executor,
          schedulerEnabled,
          schedulerConfig || null,
          trimmedWorkspace,
          worktreeEnabled,
        );
        await db.updateScheduler(todo.id, schedulerEnabled, schedulerConfig || null);
        await db.updateTodoTags(todo.id, selectedTags);
        message.success('任务已更新');
      } else {
        // Create new todo
        const newTodo = await db.createTodo(title.trim(), prompt.trim(), selectedTags);

        // If settings are configured, update them
        if (trimmedWorkspace || schedulerEnabled || executor !== 'claudecode' || worktreeEnabled) {
          if (trimmedWorkspace) {
            const exists = projectDirectories.some(d => d.path === trimmedWorkspace);
            if (!exists) {
              try {
                await db.createProjectDirectory(trimmedWorkspace);
              } catch {
                // Ignore
              }
            }
          }
          await db.updateTodo(
            newTodo.id,
            newTodo.title,
            newTodo.prompt,
            newTodo.status,
            executor,
            schedulerEnabled,
            schedulerConfig || null,
            trimmedWorkspace,
            worktreeEnabled,
          );
          await db.updateScheduler(newTodo.id, schedulerEnabled, schedulerConfig || null);
        }

        message.success('任务创建成功');
      }

      onSaved();
      onClose();
    } catch (error) {
      message.error('保存失败: ' + (error instanceof Error ? error.message : String(error)));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Drawer
      title={isEditMode ? '编辑任务' : '创建任务'}
      open={open}
      onClose={onClose}
      width={600}
      placement="right"
      destroyOnClose
      styles={{
        body: { padding: 0 },
      }}
      extra={
        <Button type="primary" loading={loading} onClick={handleSave}>
          {isEditMode ? '保存' : '创建'}
        </Button>
      }
    >
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        background: 'var(--color-bg-elevated)',
      }}>
        {/* Header with title input */}
        <div style={{ padding: '16px 20px', borderBottom: '1px solid var(--color-border-light)' }}>
          <Input
            value={title}
            onChange={e => setTitle(e.target.value)}
            placeholder="任务标题"
            style={{
              fontSize: 16,
              fontWeight: 600,
              padding: '8px 12px',
            }}
          />
        </div>

        {/* Scrollable content */}
        <div style={{ flex: 1, overflow: 'auto', padding: '16px 20px' }}>
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

          {/* Prompt Editor */}
          <div style={{ marginBottom: 16 }}>
            <div style={{ marginBottom: 10, fontWeight: 600, fontSize: 14 }}>
              <FileTextOutlined style={{ color: 'var(--color-primary)', marginRight: 6 }} />
              Prompt
            </div>
            <MdEditor
              value={prompt}
              onChange={setPrompt}
              height={200}
            />
          </div>

          <Divider style={{ margin: '8px 0 16px' }} />

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
            <AutoComplete
              value={workspace}
              onChange={(value) => setWorkspace(value)}
              options={projectDirectories.map(d => ({
                value: d.path,
                label: d.name ? `${d.name} (${d.path})` : d.path,
              }))}
              placeholder="从项目目录选择或手动输入路径"
              style={{ width: '100%' }}
              filterOption={(input, option) =>
                (option?.label as string)?.toLowerCase().includes(input.toLowerCase())
              }
            />
          </div>

          {/* Worktree Switch */}
          {(executor === 'claudecode' || executor === 'hermes') && (
            <div style={{ marginBottom: 16 }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <div style={{ fontWeight: 600, fontSize: 14 }}>
                  Git Worktree
                </div>
                <Switch
                  checked={worktreeEnabled}
                  onChange={(checked) => setWorktreeEnabled(checked)}
                  disabled={!workspace}
                />
              </div>
              {!workspace && (
                <div style={{ fontSize: 12, color: 'var(--color-text-tertiary)', marginTop: 4 }}>
                  请先设置工作目录
                </div>
              )}
            </div>
          )}

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
                <div style={{ marginTop: 12 }}>
                  <Cron
                    value={cronTo5(schedulerConfig || DEFAULT_CRON)}
                    setValue={(val: string) => setSchedulerConfig(cronTo6(val))}
                    locale={CRON_ZH_LOCALE}
                    defaultPeriod="hour"
                    humanizeLabels
                    allowClear={false}
                  />
                </div>
              </div>
            )}

            {todo?.scheduler_config && (
              <div style={{ marginTop: 8, fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                当前配置: <code>{todo.scheduler_config}</code>
              </div>
            )}
          </div>
        </div>
      </div>
    </Drawer>
  );
}
