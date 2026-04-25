import { useEffect, useState, useRef } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty, Input, Select, message, Popconfirm, Tag, Collapse, Badge } from 'antd';
import { PlayCircleOutlined, EditOutlined, DeleteOutlined, CloseCircleOutlined } from '@ant-design/icons';
import * as db from '../utils/database';
import type { LogEntry, ExecutionSummary } from '../types';
import { StatusPicker } from './StatusPicker';

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
  info: '#1890ff',
  text: '#52c41a',
  tool: '#faad14',
  step_start: '#722ed1',
  step_finish: '#13c2c2',
  stdout: '#d4d4d4',
  stderr: '#f48771',
  error: '#ff4d4f',
  system: '#909090',
  assistant: '#9c27b0',
  user: '#00bcd4',
  result: '#4caf50',
  thinking: '#ff9800',
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
  const { todos, selectedTodoId, executionRecords } = state;
  const selectedTodo = todos.find(t => t.id === selectedTodoId);

  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState('');
  const [editDescription, setEditDescription] = useState('');
  const [editStatus, setEditStatus] = useState<string>('pending');
  const [editTags, setEditTags] = useState<number[]>([]);
  const [selectedExecutor, setSelectedExecutor] = useState<string>('claudecode');
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
        selectedExecutor
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

  const handleSaveEdit = async () => {
    if (!selectedTodo) return;
    await db.updateTodo(selectedTodo.id, editTitle, editDescription, editStatus);
    await db.updateTodoTags(selectedTodo.id, editTags);
    dispatch({
      type: 'UPDATE_TODO',
      payload: { ...selectedTodo, title: editTitle, description: editDescription, status: editStatus as any, updated_at: new Date().toISOString(), tag_ids: editTags } as any
    });
    setIsEditing(false);
    message.success('更新成功');
  };

  const handleStatusChange = async (newStatus: string) => {
    if (!selectedTodo) return;
    await db.forceUpdateTodoStatus(selectedTodo.id, newStatus);
    dispatch({
      type: 'UPDATE_TODO',
      payload: { ...selectedTodo, status: newStatus as any, updated_at: new Date().toISOString() }
    });
    message.success('状态已更新');
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
        <Empty description="选择一个任务查看详情" />
      </div>
    );
  }

  return (
    <div className="detail-panel">
      {/* 标题卡片 */}
      <div className="detail-card title-card">
        {isEditing ? (
          <>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <StatusPicker
                value={editStatus}
                onChange={(val) => {
                  setEditStatus(val);
                  if (val !== selectedTodo?.status) handleStatusChange(val);
                }}
                disabled={isExecuting}
              />
              <Input
                value={editTitle}
                onChange={e => setEditTitle(e.target.value)}
                placeholder="任务标题"
                className="card-input"
                style={{ flex: 1 }}
              />
            </div>
            <TextArea
              value={editDescription}
              onChange={e => setEditDescription(e.target.value)}
              rows={3}
              placeholder="输入任务描述..."
              className="card-textarea"
            />
            {state.tags.length > 0 && (
              <Select
                value={editTags[0] || null}
                onChange={(val) => setEditTags(val ? [val] : [])}
                style={{ width: '100%' }}
                placeholder="选择标签"
                allowClear
                options={state.tags.map(tag => ({
                  value: tag.id,
                  label: (
                    <span>
                      <span style={{
                        display: 'inline-block',
                        width: 8,
                        height: 8,
                        borderRadius: '50%',
                        backgroundColor: tag.color,
                        marginRight: 8,
                      }} />
                      {tag.name}
                    </span>
                  ),
                }))}
              />
            )}
          </>
        ) : (
          <>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <StatusPicker
                value={selectedTodo.status}
                onChange={(val) => handleStatusChange(val)}
                disabled={isExecuting}
              />
              <h2 className="card-title" style={{ margin: 0 }}>{selectedTodo.title}</h2>
            </div>
            {selectedTodo.description && (
              <p className="card-description">{selectedTodo.description}</p>
            )}
          </>
        )}
      </div>

      {/* 执行按钮卡片 */}
      <div className="detail-card action-card">
        {isEditing ? (
          <div className="action-row">
            <Button onClick={() => setIsEditing(false)} className="btn-secondary btn-block">取消</Button>
            <Button type="primary" onClick={handleSaveEdit} className="btn-primary btn-block">保存</Button>
          </div>
        ) : (
          <>
            <div className="action-row">
              <Button icon={<EditOutlined />} onClick={() => setIsEditing(true)} className="btn-secondary btn-flex">
                编辑
              </Button>
              {isExecuting ? (
                <Button danger icon={<CloseCircleOutlined />} onClick={handleStopExecution} className="btn-danger btn-flex">
                  停止
                </Button>
              ) : (
                <Button type="primary" icon={<PlayCircleOutlined />} onClick={handleExecute} disabled={selectedTodo.status === 'running'} className="btn-primary btn-flex">
                  执行
                </Button>
              )}
            </div>
            <Popconfirm title="删除任务" description="确定要删除吗？" onConfirm={handleDelete}>
              <Button danger icon={<DeleteOutlined />} className="btn-danger-outline btn-block">
                删除任务
              </Button>
            </Popconfirm>
          </>
        )}
      </div>

      {/* 设置卡片 */}
      <div className="detail-card settings-card">
        <div className="setting-row">
          <span className="setting-label">执行器</span>
          <Select
            value={selectedExecutor}
            onChange={(val) => setSelectedExecutor(val)}
            disabled={isExecuting}
            className="executor-select"
            options={[
              { value: 'claudecode', label: 'Claude' },
              { value: 'joinai', label: 'JoinAI' },
            ]}
          />
        </div>
      </div>

      {summary && summary.total_executions > 0 && (
          <div className="stats-card" style={{ marginBottom: 20, flexShrink: 0 }}>
            <div style={{ fontSize: 13, color: '#999', marginBottom: 12, fontWeight: 500 }}>执行统计</div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(100px, 1fr))', gap: 12 }}>
              <div className="stats-item">
                <span className="stats-label">总执行</span>
                <span className="stats-value">{summary.total_executions}</span>
              </div>
              <div className="stats-item">
                <span className="stats-label" style={{ color: '#52c41a' }}>成功</span>
                <span className="stats-value" style={{ color: '#52c41a' }}>{summary.success_count}</span>
              </div>
              <div className="stats-item">
                <span className="stats-label" style={{ color: '#ff4d4f' }}>失败</span>
                <span className="stats-value" style={{ color: '#ff4d4f' }}>{summary.failed_count}</span>
              </div>
              {summary.total_cost_usd !== null && summary.total_cost_usd !== undefined && (
                <div className="stats-item">
                  <span className="stats-label">费用</span>
                  <span className="stats-value">${summary.total_cost_usd.toFixed(6)}</span>
                </div>
              )}
              {(totalInputTokens > 0 || totalOutputTokens > 0) && (
                <>
                  <div className="stats-item">
                    <span className="stats-label">输入Tokens</span>
                    <span className="stats-value">{totalInputTokens.toLocaleString()}</span>
                  </div>
                  <div className="stats-item">
                    <span className="stats-label">输出Tokens</span>
                    <span className="stats-value">{totalOutputTokens.toLocaleString()}</span>
                  </div>
                </>
              )}
            </div>
          </div>
        )}

        <Collapse defaultActiveKey={isExecuting ? ['realtime'] : []} style={{ marginBottom: 20, flexShrink: 0 }}>
          <Panel
            key="realtime"
            header={
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ fontWeight: 500 }}>执行过程</span>
                {isExecuting && <Badge status="processing" text="运行中" />}
                {!isExecuting && executionSuccess !== null && (
                  <Badge status={executionSuccess ? 'success' : 'error'} text={executionSuccess ? '已完成' : '已失败'} />
                )}
                {!isExecuting && executionSuccess === null && <span style={{ color: '#999' }}>暂无执行</span>}
                {realtimeLogs.length > 0 && <Tag>{realtimeLogs.length} 条日志</Tag>}
              </div>
            }
          >
            <div className="log-panel">
              {realtimeLogs.length === 0 && !isExecuting ? (
                <div style={{ color: '#666', textAlign: 'center', padding: 20 }}>暂无执行日志</div>
              ) : (
                <>
                  {realtimeLogs.map((log, idx) => (
                    <div key={idx} style={{ padding: '4px 12px', borderBottom: '1px solid #333' }}>
                      <span style={{ color: '#666', marginRight: 8 }}>{log.timestamp}</span>
                      <span style={{
                        color: logTypeColors[log.type] || '#d4d4d4',
                        background: `${logTypeColors[log.type]}20`,
                        padding: '0 6px',
                        borderRadius: 3,
                        marginRight: 8,
                        fontSize: 11,
                        fontWeight: 600,
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
                <div style={{ fontSize: 12, color: '#999', marginBottom: 6 }}>最终结果:</div>
                <div style={{
                  background: executionSuccess ? '#f6ffed' : '#fff2f0',
                  border: `1px solid ${executionSuccess ? '#b7eb8f' : '#ffccc7'}`,
                  padding: 12,
                  borderRadius: 6,
                  fontSize: 13,
                  color: '#333',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-all',
                }}>
                  {executionResult}
                </div>
              </div>
            )}
          </Panel>
        </Collapse>

        <div style={{ paddingBottom: 20, flexShrink: 0 }}>
          <h4 style={{ marginBottom: 12, fontSize: 15, fontWeight: 600 }}>执行历史</h4>
          {records.length === 0 ? (
            <Empty description="暂无执行记录" image={Empty.PRESENTED_IMAGE_SIMPLE} />
          ) : (
            records.map(record => (
              <div key={record.id} style={{
                border: '1px solid #f0f0f0',
                borderRadius: 10,
                padding: 16,
                marginBottom: 12,
                background: '#fff',
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12, flexWrap: 'wrap', gap: 8 }}>
                  <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
                    <span style={{ fontSize: 12, color: '#999' }}>{new Date(record.started_at).toLocaleString()}</span>
                    {record.executor && (
                      <Tag color={record.executor === 'claudecode' ? 'purple' : 'cyan'}>
                        {record.executor === 'claudecode' ? 'Claude' : 'JoinAI'}
                      </Tag>
                    )}
                    {record.model && <Tag color='blue'>{record.model}</Tag>}
                    {record.usage?.duration_ms && (
                      <span style={{ fontSize: 11, color: '#52c41a', fontWeight: 500 }}>
                        {(record.usage.duration_ms / 1000).toFixed(2)}s
                      </span>
                    )}
                  </div>
                  <span style={{
                    fontSize: 11,
                    padding: '2px 10px',
                    borderRadius: 12,
                    backgroundColor: record.status === 'success' ? '#52c41a' : record.status === 'failed' ? '#ff4d4f' : '#1890ff',
                    color: '#fff',
                    fontWeight: 500,
                  }}>
                    {record.status === 'success' ? '成功' : record.status === 'failed' ? '失败' : '进行中'}
                  </span>
                </div>

                {record.result !== null && record.result !== '' && (
                  <div style={{
                    background: record.status === 'success' ? '#f6ffed' : '#fff2f0',
                    border: `1px solid ${record.status === 'success' ? '#b7eb8f' : '#ffccc7'}`,
                    padding: 12,
                    borderRadius: 6,
                    fontSize: 13,
                    color: '#333',
                    whiteSpace: 'pre-wrap',
                    wordBreak: 'break-all',
                  }}>
                    {record.result}
                  </div>
                )}

                {record.usage && (
                  <div style={{ fontSize: 11, color: '#999', marginTop: 8, display: 'flex', gap: 12 }}>
                    <span>Input: {record.usage.input_tokens.toLocaleString()}</span>
                    <span>Output: {record.usage.output_tokens.toLocaleString()}</span>
                    {record.usage.total_cost_usd !== null && (
                      <span style={{ color: '#faad14' }}>${record.usage.total_cost_usd.toFixed(6)}</span>
                    )}
                  </div>
                )}

                {record.logs && record.logs !== '[]' && (
                  <details style={{ marginTop: 8 }}>
                    <summary style={{ cursor: 'pointer', color: '#1890ff', fontSize: 12 }}>
                      查看日志 ({JSON.parse(record.logs).length} 条)
                    </summary>
                    <div style={{
                      background: '#1e1e1e',
                      color: '#d4d4d4',
                      padding: 8,
                      borderRadius: 6,
                      fontFamily: 'monospace',
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
                              <span style={{ color: '#666', flexShrink: 0 }}>{log.timestamp}</span>
                              <span style={{ color: logTypeColors[log.type] || '#d4d4d4' }}>
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
    </div>
  );
}
