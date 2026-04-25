import { useEffect, useState, useRef } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty, Input, App, Popconfirm, Tag, Collapse, Badge } from 'antd';
import { PlayCircleOutlined, EditOutlined, DeleteOutlined, CloseCircleOutlined, SettingOutlined, CheckCircleOutlined } from '@ant-design/icons';
import { StatusPicker } from './StatusPicker';
import { TagCheckCardGroup } from './TagCheckCard';
import { PieChart, PieChartLegend } from './PieChart';
import { TodoSettingsModal } from './TodoSettingsModal';
import * as db from '../utils/database';
import { formatLocalDateTime } from '../utils/datetime';
import type { LogEntry, ExecutionSummary, Todo } from '../types';

const { TextArea } = Input;

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

export function TodoDetail() {
  const { state, dispatch } = useApp();
  const { message } = App.useApp();
  const { todos, selectedTodoId, executionRecords } = state;
  const selectedTodo = todos.find(t => t.id === selectedTodoId);

  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState('');
  const [editDescription, setEditDescription] = useState('');
  const [editStatus, setEditStatus] = useState<string>('pending');
  const [editTags, setEditTags] = useState<number[]>([]);
  const [summary, setSummary] = useState<ExecutionSummary | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const [isExecuting, setIsExecuting] = useState(false);
  const [currentTaskId, setCurrentTaskId] = useState<string | null>(null);
  const [realtimeLogs, setRealtimeLogs] = useState<LogEntry[]>([]);
  const [executionSuccess, setExecutionSuccess] = useState<boolean | null>(null);
  const [executionResult, setExecutionResult] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  const records = selectedTodoId ? executionRecords[selectedTodoId] || [] : [];

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
    // 只在有正在执行的任务时才建立WebSocket连接
    if (!isExecuting || !currentTaskId || !selectedTodoId) {
      return;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/xyz/events`);
    wsRef.current = ws;

    ws.onmessage = (event) => {
      if (event.data === 'Connected') return;
      try {
        const data: ExecEvent = JSON.parse(event.data);

        // 只处理当前任务的当前todo的消息
        if (data.task_id !== currentTaskId) {
          return;
        }

        if (data.type === 'Started') {
          setIsExecuting(true);
          setRealtimeLogs([]);
          setExecutionSuccess(null);
          setExecutionResult(null);
        } else if (data.type === 'Output' && data.entry) {
          setRealtimeLogs(prev => [...prev, data.entry!]);
        } else if (data.type === 'Finished') {
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
      // 连接关闭时，如果任务还在执行中，说明是异常断开
      // 这里不自动重连，等待下次状态变化触发重新连接
    };

    return () => {
      ws.close();
    };
  }, [isExecuting, currentTaskId, selectedTodoId, dispatch]);

  const handleExecute = async () => {
    if (!selectedTodo) return;
    try {
      const result = await db.executeTodo(
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
    const updated = await db.updateTodo(selectedTodo.id, selectedTodo.title, selectedTodo.description || '', newStatus);
    dispatch({
      type: 'UPDATE_TODO',
      payload: updated
    });
    message.success('状态已更新');
  };

  const handleSaveEdit = async () => {
    if (!selectedTodo) return;
    const updated = await db.updateTodo(
      selectedTodo.id,
      editTitle,
      editDescription,
      editStatus,
    );
    await db.updateTodoTags(selectedTodo.id, editTags);
    dispatch({
      type: 'UPDATE_TODO',
      payload: {
        ...updated,
        tag_ids: editTags,
      } as Todo
    });
    setIsEditing(false);
    message.success('更新成功');
  };

  const handleDelete = async () => {
    if (!selectedTodo) return;
    await db.deleteTodo(selectedTodo.id);
    dispatch({ type: 'DELETE_TODO', payload: selectedTodo.id });
    dispatch({ type: 'SELECT_TODO', payload: null });
    message.success('删除成功');
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
  const executorLabel = executor === 'claudecode' ? 'Claude' : executor === 'opencode' ? 'Opencode' : 'JoinAI';
  const executorColor = executor === 'claudecode' ? '#7c3aed' : executor === 'opencode' ? '#f59e0b' : '#0d9488';

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
                </div>
                {selectedTodo.description && (
                  <p className="card-description">{selectedTodo.description}</p>
                )}
                {/* Info tags: executor + scheduler */}
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap', marginTop: 8 }}>
                  <Tag color={executorColor} style={{ fontWeight: 600 }}>
                    {executorLabel}
                  </Tag>
                  {selectedTodo.scheduler_enabled ? (
                    <Tag color="var(--color-primary)" style={{ fontWeight: 600 }}>
                      调度: {selectedTodo.scheduler_config}
                    </Tag>
                  ) : (
                    <Tag style={{ fontWeight: 600, color: 'var(--color-text-tertiary)', borderColor: 'var(--color-border)' }}>
                      调度: 关闭
                    </Tag>
                  )}
                  {records.length > 0 && (
                    <span style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                      上次: {formatLocalDateTime(records[0].started_at)}
                    </span>
                  )}
                  {selectedTodo.scheduler_next_run_at && (
                    <span style={{ fontSize: 12, color: 'var(--color-success)' }}>
                      下次: {formatLocalDateTime(selectedTodo.scheduler_next_run_at)}
                    </span>
                  )}
                </div>
              </div>
              <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
                <Button
                  type="text"
                  icon={<SettingOutlined />}
                  onClick={() => setSettingsOpen(true)}
                  className="icon-btn"
                  aria-label="任务设置"
                />
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

      {/* Execution Stats */}
      {summary && summary.total_executions > 0 && (
        <div className="detail-card" style={{ padding: '16px 20px' }}>
          {(() => {
            const input = summary.total_input_tokens;
            const output = summary.total_output_tokens;
            const cacheRead = (summary as any).total_cache_read_tokens ?? 0;
            const cacheCreate = (summary as any).total_cache_creation_tokens ?? 0;
            const totalTokens = input + output + cacheRead + cacheCreate;

            const tokenSegments = [
              { value: input, color: '#3b82f6', label: '输入' },
              { value: output, color: '#22c55e', label: '输出' },
              { value: cacheRead, color: '#f59e0b', label: '缓存读' },
              { value: cacheCreate, color: '#a78bfa', label: '缓存写' },
            ];

            return (
              <div>
                {/* Top row: pie + big number */}
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 20,
                    flexWrap: 'wrap',
                    marginBottom: 12,
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                    <PieChart
                      segments={tokenSegments.filter((s) => s.value > 0)}
                      size={90}
                    />
                    <div>
                      <div
                        style={{
                          fontSize: 28,
                          fontWeight: 700,
                          color: 'var(--color-text)',
                          lineHeight: 1.2,
                          letterSpacing: '-0.02em',
                        }}
                      >
                        {totalTokens > 0
                          ? totalTokens.toLocaleString()
                          : '0'}
                      </div>
                      <div
                        style={{
                          fontSize: 12,
                          color: 'var(--color-text-tertiary)',
                          fontWeight: 500,
                        }}
                      >
                        Tokens
                      </div>
                    </div>
                  </div>
                  <div style={{ flex: 1, minWidth: 180 }}>
                    <PieChartLegend segments={tokenSegments} />
                  </div>
                </div>

                {/* Bottom row: execution summary + cost */}
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 12,
                    flexWrap: 'wrap',
                    paddingTop: 12,
                    borderTop: '1px solid var(--color-border-light)',
                    fontSize: 12,
                    color: 'var(--color-text-secondary)',
                  }}
                >
                  <span>
                    执行{' '}
                    <strong style={{ color: 'var(--color-text)' }}>
                      {summary.total_executions}
                    </strong>{' '}
                    次
                  </span>
                  <span style={{ color: 'var(--color-border)' }}>|</span>
                  <span style={{ color: 'var(--color-success)' }}>
                    成功 {summary.success_count}
                  </span>
                  <span style={{ color: 'var(--color-error)' }}>
                    失败 {summary.failed_count}
                  </span>
                  {summary.total_cost_usd !== null &&
                    summary.total_cost_usd !== undefined && (
                      <>
                        <span style={{ color: 'var(--color-border)' }}>|</span>
                        <span style={{ color: 'var(--color-warning)', fontWeight: 600 }}>
                          ${summary.total_cost_usd.toFixed(4)}
                        </span>
                      </>
                    )}
                </div>
              </div>
            );
          })()}
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
              disabled={isExecuting}
              block
              className="btn-execute"
            >
              执行任务
            </Button>
          )}
        </div>
      )}

      {/* Realtime Logs */}
      <Collapse
        defaultActiveKey={isExecuting ? ['realtime'] : []}
        style={{ marginBottom: 12, flexShrink: 0 }}
        items={[
          {
            key: 'realtime',
            label: (
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ fontWeight: 600 }}>执行过程</span>
                {isExecuting && <Badge status="processing" text="运行中" />}
                {!isExecuting && executionSuccess !== null && (
                  <Badge status={executionSuccess ? 'success' : 'error'} text={executionSuccess ? '已完成' : '已失败'} />
                )}
                {!isExecuting && executionSuccess === null && <span style={{ color: 'var(--color-text-tertiary)' }}>暂无执行</span>}
                {realtimeLogs.length > 0 && <Tag color="var(--color-primary)">{realtimeLogs.length} 条日志</Tag>}
              </div>
            ),
            children: (
              <>
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
              </>
            ),
          },
        ]}
      />

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
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8, flexWrap: 'wrap', gap: 8 }}>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
                  <span style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                    {formatLocalDateTime(record.started_at)}
                  </span>
                  {record.executor && (
                    <Tag color={record.executor === 'claudecode' ? '#7c3aed' : record.executor === 'opencode' ? '#f59e0b' : '#0d9488'} style={{ fontWeight: 600 }}>
                      {record.executor === 'claudecode' ? 'Claude' : record.executor === 'opencode' ? 'Opencode' : 'JoinAI'}
                    </Tag>
                  )}
                  {record.model && <Tag color="#3b82f6">{record.model}</Tag>}
                  <Tag color={record.trigger_type === 'cron' ? '#8b5cf6' : '#6b7280'} style={{ fontSize: 10 }}>
                    {record.trigger_type === 'cron' ? 'Cron' : '手动'}
                  </Tag>
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
              {record.command && (
                <div style={{ fontSize: 11, color: 'var(--color-text-quaternary)', marginBottom: 8, fontFamily: 'var(--font-mono)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {record.command}
                </div>
              )}

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

      <TodoSettingsModal
        open={settingsOpen}
        todo={selectedTodo}
        onClose={() => setSettingsOpen(false)}
        onUpdated={() => {
          db.getAllTodos().then(todos => {
            dispatch({ type: 'SET_TODOS', payload: todos });
          });
          if (selectedTodoId) {
            db.getExecutionSummary(selectedTodoId).then(sum => {
              setSummary(sum);
            });
          }
        }}
      />
    </div>
  );
}
