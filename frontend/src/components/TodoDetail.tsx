import { useEffect, useState, useRef } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty, Input, Select, message, Popconfirm, Tag, Collapse, Badge, Switch, Tooltip, Divider } from 'antd';
import { PlayCircleOutlined, EditOutlined, DeleteOutlined, CloseCircleOutlined, ClockCircleOutlined, InfoCircleOutlined, CheckCircleOutlined } from '@ant-design/icons';
import { StatisticCard } from '@ant-design/pro-components';
import { StatusPicker } from './StatusPicker';
import { TagCheckCardGroup } from './TagCheckCard';
import * as db from '../utils/database';
import type { LogEntry, ExecutionSummary } from '../types';

const { TextArea } = Input;
const { Panel } = Collapse;

interface ExecEvent {
  type: 'Started' | 'Output' | 'Finished';
  task_id: string;
  entry?: LogEntry;
  success?: boolean;
  result?: string | null;
}

const logTypeColors: Record<string, string> = {
  info: '#60a5fa',
  text: '#4ade80',
  tool: '#fbbf24',
  step_start: '#c084fc',
  step_finish: '#2dd4bf',
  stdout: '#cbd5e1',
  stderr: '#f87171',
  error: '#ef4444',
  system: '#94a3b8',
  assistant: '#a78bfa',
  user: '#22d3ee',
  result: '#4ade80',
  thinking: '#fb923c',
};

const logTypeLabels: Record<string, string> = {
  info: 'INFO',
  text: 'TEXT',
  tool: 'TOOL',
  step_start: 'START',
  step_finish: 'END',
  stdout: 'OUT',
  stderr: 'ERR',
  error: 'ERROR',
  system: 'SYS',
  assistant: 'ASST',
  user: 'USER',
  result: 'RESULT',
  thinking: 'THINK',
};

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

export function TodoDetail() {
  const { state, dispatch } = useApp();
  const { todos, selectedTodoId, executionRecords } = state;
  const selectedTodo = todos.find(t => t.id === selectedTodoId);

  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState('');
  const [editDescription, setEditDescription] = useState('');
  const [editStatus, setEditStatus] = useState<string>('pending');
  const [editTags, setEditTags] = useState<number[]>([]);
  const [editExecutor, setEditExecutor] = useState<string>('claudecode');
  const [editSchedulerEnabled, setEditSchedulerEnabled] = useState(false);
  const [editSchedulerConfig, setEditSchedulerConfig] = useState<string>('');
  const [editCronPreset, setEditCronPreset] = useState<string>('custom');
  const [summary, setSummary] = useState<ExecutionSummary | null>(null);

  const [isExecuting, setIsExecuting] = useState(false);
  const [currentTaskId, setCurrentTaskId] = useState<string | null>(null);
  const [realtimeLogs, setRealtimeLogs] = useState<LogEntry[]>([]);
  const [executionSuccess, setExecutionSuccess] = useState<boolean | null>(null);
  const [executionResult, setExecutionResult] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  const records = selectedTodoId ? executionRecords[selectedTodoId] || [] : [];

  const totalInputTokens = records.reduce((sum, record) => sum + (record.usage?.input_tokens || 0), 0);
  const totalOutputTokens = records.reduce((sum, record) => sum + (record.usage?.output_tokens || 0), 0);

  useEffect(() => {
    if (logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [realtimeLogs]);

  useEffect(() => {
    if (selectedTodo) {
      setEditTitle(selectedTodo.title);
      setEditDescription(selectedTodo.description || '');
      setEditStatus(selectedTodo.status);
      setEditTags((selectedTodo as any).tag_ids || []);
      setEditExecutor(selectedTodo.executor || 'claudecode');
      setEditSchedulerEnabled(selectedTodo.scheduler_enabled || false);
      setEditSchedulerConfig(selectedTodo.scheduler_config || '');

      db.getExecutionRecords(selectedTodo.id).then(recs => {
        dispatch({
          type: 'SET_EXECUTION_RECORDS',
          payload: { todoId: selectedTodo.id, records: recs }
        });
      });

      db.getExecutionSummary(selectedTodo.id).then(sum => {
        setSummary(sum);
      });
    }
  }, [selectedTodoId, selectedTodo, dispatch]);

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/xyz/events`);
    wsRef.current = ws;

    ws.onmessage = (event) => {
      if (event.data === 'Connected') return;
      try {
        const data: ExecEvent = JSON.parse(event.data);

        if (data.type === 'Started' && data.task_id === currentTaskId) {
          setIsExecuting(true);
          setRealtimeLogs([]);
          setExecutionSuccess(null);
          setExecutionResult(null);
        } else if (data.type === 'Output' && data.entry && data.task_id === currentTaskId) {
          setRealtimeLogs(prev => [...prev, data.entry!]);
        } else if (data.type === 'Finished' && data.task_id === currentTaskId) {
          setIsExecuting(false);
          setExecutionSuccess(data.success ?? null);
          setExecutionResult(data.result ?? null);
          message.success(data.success ? '执行成功' : '执行失败');

          db.getAllTodos().then(todos => {
            dispatch({ type: 'SET_TODOS', payload: todos });
          });

          if (selectedTodoId) {
            db.getExecutionRecords(selectedTodoId).then(recs => {
              dispatch({
                type: 'SET_EXECUTION_RECORDS',
                payload: { todoId: selectedTodoId, records: recs }
              });
            });
            db.getExecutionSummary(selectedTodoId).then(sum => {
              setSummary(sum);
            });
          }
        }
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e);
      }
    };

    ws.onclose = () => {
      setTimeout(() => {
        if (wsRef.current?.readyState !== WebSocket.OPEN) {
          // reconnect logic handled by component re-mount
        }
      }, 1000);
    };

    return () => {
      ws.close();
    };
  }, [currentTaskId, selectedTodoId, dispatch]);

  const handleExecute = async () => {
    if (!selectedTodo) return;
    try {
      const result = await db.executeJoinai(
        selectedTodo.id,
        selectedTodo.description || selectedTodo.title,
        selectedTodo.executor || undefined
      );
      setCurrentTaskId(result.task_id);
      setIsExecuting(true);
      setRealtimeLogs([]);
      setExecutionSuccess(null);
      setExecutionResult(null);
    } catch (error) {
      message.error('执行失败: ' + error);
    }
  };

  const handleStopExecution = () => {
    setIsExecuting(false);
    setCurrentTaskId(null);
    message.info('已停止执行');
  };

  const handleStatusChange = async (newStatus: string) => {
    if (!selectedTodo) return;
    await db.updateTodo(selectedTodo.id, selectedTodo.title, selectedTodo.description || '', newStatus);
    dispatch({
      type: 'UPDATE_TODO',
      payload: { ...selectedTodo, status: newStatus as any, updated_at: new Date().toISOString() }
    });
    message.success('状态已更新');
  };

  const handleSaveEdit = async () => {
    if (!selectedTodo) return;
    await db.updateTodo(
      selectedTodo.id,
      editTitle,
      editDescription,
      editStatus,
      editExecutor,
      editSchedulerEnabled,
      editSchedulerConfig || null,
    );
    await db.updateTodoTags(selectedTodo.id, editTags);
    dispatch({
      type: 'UPDATE_TODO',
      payload: {
        ...selectedTodo,
        title: editTitle,
        description: editDescription,
        status: editStatus as any,
        executor: editExecutor,
        scheduler_enabled: editSchedulerEnabled,
        scheduler_config: editSchedulerConfig || null,
        updated_at: new Date().toISOString(),
        tag_ids: editTags,
      } as any
    });
    setIsEditing(false);
    message.success('更新成功');
  };

  const handleUpdateScheduler = async () => {
    if (!selectedTodo) return;
    try {
      const res = await fetch(`/xyz/todos/${selectedTodo.id}/scheduler`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          scheduler_enabled: editSchedulerEnabled,
          scheduler_config: editSchedulerConfig || null,
        }),
      });
      if (!res.ok) throw new Error('Failed to update scheduler');
      const updatedTodo = await res.json();
      dispatch({
        type: 'UPDATE_TODO',
        payload: { ...selectedTodo, ...updatedTodo, updated_at: new Date().toISOString() }
      });
      message.success('调度设置已更新');
    } catch (error) {
      message.error('调度更新失败: ' + error);
    }
  };

  const handleDelete = async () => {
    if (!selectedTodo) return;
    await db.deleteTodo(selectedTodo.id);
    dispatch({ type: 'DELETE_TODO', payload: selectedTodo.id });
    dispatch({ type: 'SELECT_TODO', payload: null });
    message.success('删除成功');
  };

  const handleCronPresetChange = (value: string) => {
    setEditCronPreset(value);
    if (value !== 'custom') {
      setEditSchedulerConfig(value);
    }
  };

  if (!selectedTodo) {
    return (
      <div className="detail-panel" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <div className="empty-state">
          <div className="empty-state-icon">
            <CheckCircleOutlined />
          </div>
          <Empty
            description={
              <div style={{ color: 'var(--color-text-tertiary)', fontSize: 14 }}>
                选择一个任务查看详情
              </div>
            }
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        </div>
      </div>
    );
  }

  const executor = selectedTodo.executor || 'claudecode';
  const executorLabel = executor === 'claudecode' ? 'Claude' : 'JoinAI';
  const executorColor = executor === 'claudecode' ? '#7c3aed' : '#0d9488';

  return (
    <div className="detail-panel">
      {/* Title Card */}
      <div className="detail-card title-card">
        {isEditing ? (
          <>
            <Input
              value={editTitle}
              onChange={e => setEditTitle(e.target.value)}
              placeholder="任务标题"
              className="card-input"
              style={{ marginBottom: 12 }}
            />
            <TextArea
              value={editDescription}
              onChange={e => setEditDescription(e.target.value)}
              rows={3}
              placeholder="输入任务描述..."
              className="card-textarea"
              style={{ marginBottom: 12 }}
            />
            <Select
              value={editExecutor}
              onChange={(val) => setEditExecutor(val)}
              style={{ width: '100%', marginBottom: 12 }}
              placeholder="选择执行器"
              options={[
                { value: 'claudecode', label: 'Claude' },
                { value: 'joinai', label: 'JoinAI' },
              ]}
            />
            {state.tags.length > 0 && (
              <div style={{ marginBottom: 12 }}>
                <div style={{ marginBottom: 8, fontWeight: 600 }}>标签</div>
                <TagCheckCardGroup
                  tags={state.tags}
                  value={editTags[0] || null}
                  onChange={(val) => setEditTags(val ? [val as number] : [])}
                />
              </div>
            )}
            <div style={{ display: 'flex', gap: 8 }}>
              <Button onClick={() => setIsEditing(false)} block>取消</Button>
              <Button type="primary" onClick={handleSaveEdit} block>保存</Button>
            </div>
          </>
        ) : (
          <>
            <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 8 }}>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8, flexWrap: 'wrap' }}>
                  <StatusPicker
                    value={selectedTodo.status}
                    onChange={handleStatusChange}
                    disabled={isExecuting}
                  />
                  <h2 className="card-title" style={{ margin: 0 }}>{selectedTodo.title}</h2>
                  <Tag color={executorColor} style={{ fontWeight: 600 }}>
                    {executorLabel}
                  </Tag>
                </div>
                {selectedTodo.description && (
                  <p className="card-description">{selectedTodo.description}</p>
                )}
              </div>
              <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
                <Button
                  type="text"
                  icon={<EditOutlined />}
                  onClick={() => setIsEditing(true)}
                  className="icon-btn"
                  aria-label="编辑任务"
                />
                <Popconfirm title="删除任务" description="确定要删除吗？" onConfirm={handleDelete}>
                  <Button
                    type="text"
                    danger
                    icon={<DeleteOutlined />}
                    className="icon-btn"
                    aria-label="删除任务"
                  />
                </Popconfirm>
              </div>
            </div>
          </>
        )}
      </div>

      {/* Settings Card */}
      {!isEditing && (
        <div className="detail-card settings-card">
          <div className="setting-row">
            <span className="setting-label">执行器</span>
            <Tag color={executorColor} style={{ fontWeight: 600 }}>
              {executorLabel}
            </Tag>
          </div>

          <Divider style={{ margin: '12px 0' }} />

          <div className="setting-row" style={{ flexDirection: 'column', alignItems: 'flex-start', gap: 12 }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
              <span className="setting-label" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <ClockCircleOutlined style={{ color: 'var(--color-primary)' }} />
                定时调度
                <Tooltip title="使用 Cron 表达式设置周期性自动执行">
                  <InfoCircleOutlined style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }} />
                </Tooltip>
              </span>
              <Switch
                checked={editSchedulerEnabled}
                onChange={(checked) => {
                  setEditSchedulerEnabled(checked);
                  if (checked && !editSchedulerConfig) {
                    setEditSchedulerConfig('0 */10 * * * *');
                    setEditCronPreset('0 */10 * * * *');
                  }
                }}
              />
            </div>

            <div style={{ width: '100%' }}>
              <Select
                value={editCronPreset}
                onChange={handleCronPresetChange}
                style={{ width: '100%', marginBottom: 8 }}
                placeholder="选择预设或自定义"
                options={cronPresets}
                disabled={!editSchedulerEnabled}
              />
              {editCronPreset === 'custom' && (
                <Input
                  value={editSchedulerConfig}
                  onChange={e => setEditSchedulerConfig(e.target.value)}
                  placeholder="Cron 表达式，例如: 0 */10 * * * *"
                  disabled={!editSchedulerEnabled}
                  style={{ marginBottom: 8 }}
                />
              )}
              <Button
                type="primary"
                size="small"
                onClick={handleUpdateScheduler}
                disabled={!editSchedulerConfig}
                block
              >
                保存调度设置
              </Button>
            </div>

            {selectedTodo.scheduler_config && (
              <div style={{ fontSize: 12, color: 'var(--color-text-tertiary)', width: '100%' }}>
                <div>当前配置: <code>{selectedTodo.scheduler_config}</code></div>
                <div style={{ marginTop: 4 }}>
                  状态:{' '}
                  <span style={{
                    color: selectedTodo.scheduler_enabled ? 'var(--color-success)' : 'var(--color-text-tertiary)',
                    fontWeight: 600,
                  }}>
                    {selectedTodo.scheduler_enabled ? '已启用' : '已禁用'}
                  </span>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Action Card */}
      {!isEditing && (
        <div className="detail-card action-card">
          {isExecuting ? (
            <Button
              danger
              icon={<CloseCircleOutlined />}
              onClick={handleStopExecution}
              block
              className="btn-stop"
            >
              停止执行
            </Button>
          ) : (
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              onClick={handleExecute}
              disabled={selectedTodo.status === 'running'}
              block
              className="btn-execute"
            >
              {selectedTodo.status === 'running' ? '执行中...' : '执行任务'}
            </Button>
          )}
        </div>
      )}

      {/* Stats Card */}
      {summary && summary.total_executions > 0 && (
        <StatisticCard.Group style={{ marginBottom: 12, flexShrink: 0 }}>
          <StatisticCard
            statistic={{
              title: '总执行',
              value: summary.total_executions,
            }}
          />
          <StatisticCard
            statistic={{
              title: '成功',
              value: summary.success_count,
              description: (
                <span style={{ color: 'var(--color-success)', fontSize: 12 }}>
                  {((summary.success_count / summary.total_executions) * 100).toFixed(1)}%
                </span>
              ),
            }}
          />
          <StatisticCard
            statistic={{
              title: '失败',
              value: summary.failed_count,
              description: (
                <span style={{ color: 'var(--color-error)', fontSize: 12 }}>
                  {((summary.failed_count / summary.total_executions) * 100).toFixed(1)}%
                </span>
              ),
            }}
          />
          {summary.total_cost_usd !== null && summary.total_cost_usd !== undefined && (
            <StatisticCard
              statistic={{
                title: '费用',
                value: `$${summary.total_cost_usd.toFixed(4)}`,
              }}
            />
          )}
          {(totalInputTokens > 0 || totalOutputTokens > 0) && (
            <StatisticCard
              statistic={{
                title: 'Tokens',
                value: `${(totalInputTokens + totalOutputTokens).toLocaleString()}`,
                description: (
                  <span style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                    In: {totalInputTokens.toLocaleString()} / Out: {totalOutputTokens.toLocaleString()}
                  </span>
                ),
              }}
            />
          )}
        </StatisticCard.Group>
      )}

      {/* Realtime Logs */}
      <Collapse defaultActiveKey={isExecuting ? ['realtime'] : []} style={{ marginBottom: 12, flexShrink: 0 }}>
        <Panel
          key="realtime"
          header={
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ fontWeight: 600 }}>执行过程</span>
              {isExecuting && <Badge status="processing" text="运行中" />}
              {!isExecuting && executionSuccess !== null && (
                <Badge status={executionSuccess ? 'success' : 'error'} text={executionSuccess ? '已完成' : '已失败'} />
              )}
              {!isExecuting && executionSuccess === null && <span style={{ color: 'var(--color-text-tertiary)' }}>暂无执行</span>}
              {realtimeLogs.length > 0 && <Tag color="var(--color-primary)">{realtimeLogs.length} 条日志</Tag>}
            </div>
          }
        >
          <div className="log-panel" style={{ maxHeight: 400, overflow: 'auto' }}>
            {realtimeLogs.length === 0 && !isExecuting ? (
              <div style={{ color: 'var(--color-text-tertiary)', textAlign: 'center', padding: 24 }}>
                暂无执行日志
              </div>
            ) : (
              <>
                {realtimeLogs.map((log, idx) => (
                  <div key={idx} style={{ padding: '4px 12px', borderBottom: '1px solid rgba(255,255,255,0.06)' }}>
                    <span className="log-timestamp">{log.timestamp}</span>
                    <span style={{
                      color: logTypeColors[log.type] || '#cbd5e1',
                      background: `${logTypeColors[log.type]}20`,
                      padding: '1px 6px',
                      borderRadius: 3,
                      marginRight: 8,
                      fontSize: 10,
                      fontWeight: 700,
                    }}>
                      {logTypeLabels[log.type] || log.type}
                    </span>
                    <span style={{ wordBreak: 'break-all', whiteSpace: 'pre-wrap' }}>{log.content}</span>
                  </div>
                ))}
                <div ref={logsEndRef} />
              </>
            )}
          </div>

          {executionResult !== null && executionResult !== '' && (
            <div style={{ marginTop: 12, padding: '0 12px 12px' }}>
              <div style={{ fontSize: 12, color: 'var(--color-text-tertiary)', marginBottom: 6, fontWeight: 600 }}>最终结果</div>
              <div style={{
                background: executionSuccess ? 'var(--color-success-bg)' : 'var(--color-error-bg)',
                border: `1px solid ${executionSuccess ? '#bbf7d0' : '#fecaca'}`,
                padding: 12,
                borderRadius: 8,
                fontSize: 13,
                color: 'var(--color-text)',
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-all',
              }}>
                {executionResult}
              </div>
            </div>
          )}
        </Panel>
      </Collapse>

      {/* Execution History */}
      <div style={{ paddingBottom: 20, flexShrink: 0 }}>
        <h4 style={{ marginBottom: 12, fontSize: 15, fontWeight: 700, color: 'var(--color-text)' }}>执行历史</h4>
        {records.length === 0 ? (
          <Empty description="暂无执行记录" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        ) : (
          records.map(record => (
            <div
              key={record.id}
              className={`history-card history-card-${record.status}`}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12, flexWrap: 'wrap', gap: 8 }}>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
                  <span style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                    {new Date(record.started_at).toLocaleString()}
                  </span>
                  {record.executor && (
                    <Tag color={record.executor === 'claudecode' ? '#7c3aed' : '#0d9488'} style={{ fontWeight: 600 }}>
                      {record.executor === 'claudecode' ? 'Claude' : 'JoinAI'}
                    </Tag>
                  )}
                  {record.model && <Tag color="#3b82f6">{record.model}</Tag>}
                  {record.usage?.duration_ms && (
                    <span style={{ fontSize: 11, color: 'var(--color-success)', fontWeight: 600 }}>
                      {(record.usage.duration_ms / 1000).toFixed(2)}s
                    </span>
                  )}
                </div>
                <span style={{
                  fontSize: 11,
                  padding: '3px 12px',
                  borderRadius: 12,
                  backgroundColor: record.status === 'success' ? 'var(--color-success)' : record.status === 'failed' ? 'var(--color-error)' : 'var(--color-info)',
                  color: '#fff',
                  fontWeight: 600,
                }}>
                  {record.status === 'success' ? '成功' : record.status === 'failed' ? '失败' : '进行中'}
                </span>
              </div>

              {record.result !== null && record.result !== '' && (
                <div className={`history-result ${record.status === 'success' ? 'history-result-success' : 'history-result-failed'}`}>
                  {record.result}
                </div>
              )}

              {record.usage && (
                <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginTop: 8, display: 'flex', gap: 12, flexWrap: 'wrap' }}>
                  <span>Input: {record.usage.input_tokens.toLocaleString()}</span>
                  <span>Output: {record.usage.output_tokens.toLocaleString()}</span>
                  {record.usage.total_cost_usd !== null && (
                    <span style={{ color: 'var(--color-warning)', fontWeight: 600 }}>${record.usage.total_cost_usd.toFixed(6)}</span>
                  )}
                </div>
              )}

              {record.logs && record.logs !== '[]' && (
                <details style={{ marginTop: 8 }}>
                  <summary style={{ cursor: 'pointer', color: 'var(--color-primary)', fontSize: 12, fontWeight: 600 }}>
                    查看日志 ({JSON.parse(record.logs).length} 条)
                  </summary>
                  <div style={{
                    background: '#1a1a2e',
                    color: '#e2e8f0',
                    padding: 8,
                    borderRadius: 8,
                    fontFamily: 'var(--font-mono)',
                    fontSize: 11,
                    marginTop: 8,
                    maxHeight: 250,
                    overflow: 'auto',
                  }}>
                    {(() => {
                      try {
                        const logs = JSON.parse(record.logs) as LogEntry[];
                        return logs.map((log, idx) => (
                          <div key={idx} style={{ marginBottom: 4, display: 'flex', gap: 8 }}>
                            <span style={{ color: '#64748b', flexShrink: 0 }}>{log.timestamp}</span>
                            <span style={{ color: logTypeColors[log.type] || '#cbd5e1' }}>
                              [{logTypeLabels[log.type] || log.type}]
                            </span>
                            <span>{log.content}</span>
                          </div>
                        ));
                      } catch {
                        return <div>{record.logs}</div>;
                      }
                    })()}
                  </div>
                </details>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
