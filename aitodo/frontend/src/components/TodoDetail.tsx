import { useEffect, useState, useRef } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty, Input, Select, message, Popconfirm, Tag, Collapse, Badge, Tooltip } from 'antd';
import { PlayCircleOutlined, EditOutlined, DeleteOutlined, CloseCircleOutlined, CheckCircleOutlined, ExclamationCircleOutlined } from '@ant-design/icons';
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
  const [selectedExecutor, setSelectedExecutor] = useState<string>('joinai');
  const [summary, setSummary] = useState<ExecutionSummary | null>(null);

  // Current execution state
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentTaskId, setCurrentTaskId] = useState<string | null>(null);
  const [realtimeLogs, setRealtimeLogs] = useState<LogEntry[]>([]);
  const [executionSuccess, setExecutionSuccess] = useState<boolean | null>(null);
  const [executionResult, setExecutionResult] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  const records = selectedTodoId ? executionRecords[selectedTodoId] || [] : [];

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [realtimeLogs]);

  // Fetch execution records and summary when todo changes
  useEffect(() => {
    if (selectedTodo) {
      setEditTitle(selectedTodo.title);
      setEditDescription(selectedTodo.description || '');
      setEditStatus(selectedTodo.status);

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

  // Setup WebSocket connection
  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/api/events`);
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

          // Reload todos and execution records and summary
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
      // Reconnect after a delay if not intentionally closed
      setTimeout(() => {
        if (wsRef.current?.readyState !== WebSocket.OPEN) {
          // Will be handled by next useEffect cycle
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
      console.error('Execution failed:', error);
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
    dispatch({
      type: 'UPDATE_TODO',
      payload: { ...selectedTodo, title: editTitle, description: editDescription, status: editStatus as any, updated_at: new Date().toISOString() }
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
      <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Empty description="选择一个 Todo 查看详情" />
      </div>
    );
  }

  return (
    <div style={{ flex: 1, padding: 24, overflow: 'auto' }}>
      {/* Header */}
      <div style={{ marginBottom: 24 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 16 }}>
          {isEditing ? (
            <Input
              value={editTitle}
              onChange={e => setEditTitle(e.target.value)}
              style={{ fontSize: 20, fontWeight: 600, width: 300 }}
            />
          ) : (
            <h2 style={{ margin: 0 }}>{selectedTodo.title}</h2>
          )}
          <div style={{ display: 'flex', gap: 8 }}>
            {isEditing ? (
              <>
                <Button onClick={() => setIsEditing(false)}>取消</Button>
                <Button type="primary" onClick={handleSaveEdit}>保存</Button>
              </>
            ) : (
              <>
                <Button icon={<EditOutlined />} onClick={() => setIsEditing(true)}>编辑</Button>
                {isExecuting ? (
                  <Button
                    danger
                    icon={<CloseCircleOutlined />}
                    onClick={handleStopExecution}
                  >
                    停止
                  </Button>
                ) : (
                  <Button
                    type="primary"
                    icon={<PlayCircleOutlined />}
                    onClick={handleExecute}
                    disabled={selectedTodo.status === 'running'}
                  >
                    执行
                  </Button>
                )}
                <Popconfirm
                  title="删除 Todo"
                  description="确定要删除这个 Todo 吗？"
                  onConfirm={handleDelete}
                >
                  <Button icon={<DeleteOutlined />} danger>删除</Button>
                </Popconfirm>
              </>
            )}
          </div>
        </div>

        <div style={{ display: 'flex', gap: 16, alignItems: 'center' }}>
          <Select
            value={editStatus}
            onChange={(val) => {
              setEditStatus(val);
              if (val !== selectedTodo?.status) {
                handleStatusChange(val);
              }
            }}
            style={{ width: 120 }}
            options={[
              { value: 'pending', label: '待执行' },
              { value: 'running', label: '执行中' },
              { value: 'completed', label: '已完成' },
              { value: 'failed', label: '执行失败' },
            ]}
          />
          <Tooltip title="强制修改状态（当进程崩溃时使用）">
            <Button
              type="text"
              size="small"
              icon={<ExclamationCircleOutlined />}
              onClick={() => {
                const newStatus = editStatus === 'running' ? 'failed' : 'running';
                setEditStatus(newStatus);
                handleStatusChange(newStatus);
              }}
            >
              强制
            </Button>
          </Tooltip>

          <span style={{ fontSize: 12, color: '#999' }}>执行器:</span>
          <Select
            value={selectedExecutor}
            onChange={setSelectedExecutor}
            style={{ width: 140 }}
            disabled={isExecuting}
            options={[
              { value: 'joinai', label: 'JoinAI' },
              { value: 'claudecode', label: 'Claude Code' },
            ]}
          />

          {isExecuting && (
            <Tag color="blue" icon={<PlayCircleOutlined />}>执行中</Tag>
          )}
          {executionSuccess === true && (
            <Tag color="green" icon={<CheckCircleOutlined />}>执行成功</Tag>
          )}
          {executionSuccess === false && (
            <Tag color="red" icon={<CloseCircleOutlined />}>执行失败</Tag>
          )}
        </div>
      </div>

      {/* Description */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: '#999', marginBottom: 4 }}>描述</div>
        {isEditing ? (
          <TextArea
            value={editDescription}
            onChange={e => setEditDescription(e.target.value)}
            rows={3}
            placeholder="输入描述..."
          />
        ) : (
          <div style={{ color: '#666' }}>
            {selectedTodo.description || '无描述'}
          </div>
        )}
      </div>

      {/* Execution Summary */}
      {summary && summary.total_executions > 0 && (
        <div style={{ marginBottom: 16, padding: 12, background: '#f5f5f5', borderRadius: 8 }}>
          <div style={{ fontSize: 12, color: '#999', marginBottom: 8 }}>执行统计</div>
          <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap' }}>
            <div>
              <span style={{ color: '#666' }}>总执行:</span>
              <span style={{ fontWeight: 600, marginLeft: 4 }}>{summary.total_executions}</span>
            </div>
            <div>
              <span style={{ color: '#52c41a' }}>成功:</span>
              <span style={{ fontWeight: 600, marginLeft: 4, color: '#52c41a' }}>{summary.success_count}</span>
            </div>
            <div>
              <span style={{ color: '#ff4d4f' }}>失败:</span>
              <span style={{ fontWeight: 600, marginLeft: 4, color: '#ff4d4f' }}>{summary.failed_count}</span>
            </div>
            {summary.running_count > 0 && (
              <div>
                <span style={{ color: '#1890ff' }}>进行中:</span>
                <span style={{ fontWeight: 600, marginLeft: 4, color: '#1890ff' }}>{summary.running_count}</span>
              </div>
            )}
            {summary.total_cost_usd !== null && summary.total_cost_usd !== undefined && (
              <div>
                <span style={{ color: '#666' }}>总费用:</span>
                <span style={{ fontWeight: 600, marginLeft: 4 }}>${summary.total_cost_usd.toFixed(6)}</span>
              </div>
            )}
            <div>
              <span style={{ color: '#666' }}>Input:</span>
              <span style={{ marginLeft: 4 }}>{summary.total_input_tokens.toLocaleString()}</span>
            </div>
            <div>
              <span style={{ color: '#666' }}>Output:</span>
              <span style={{ marginLeft: 4 }}>{summary.total_output_tokens.toLocaleString()}</span>
            </div>
          </div>
        </div>
      )}

      {/* Realtime Execution Panel */}
      <Collapse defaultActiveKey={isExecuting ? ['realtime'] : []} style={{ marginBottom: 24 }}>
        <Panel
          key="realtime"
          header={
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span>执行过程</span>
              {isExecuting && <Badge status="processing" text="运行中" />}
              {!isExecuting && executionSuccess !== null && (
                <Badge status={executionSuccess ? 'success' : 'error'} text={executionSuccess ? '已完成' : '已失败'} />
              )}
              {!isExecuting && executionSuccess === null && <span style={{ color: '#999' }}>暂无执行</span>}
              {realtimeLogs.length > 0 && <Tag>{realtimeLogs.length} 条日志</Tag>}
            </div>
          }
        >
          <div style={{
            background: '#1e1e1e',
            color: '#d4d4d4',
            padding: 12,
            borderRadius: 6,
            fontFamily: 'monospace',
            fontSize: 12,
            maxHeight: 500,
            overflow: 'auto',
          }}>
            {realtimeLogs.length === 0 && !isExecuting ? (
              <div style={{ color: '#666', textAlign: 'center', padding: 20 }}>
                暂无执行日志
              </div>
            ) : (
              <>
                {realtimeLogs.map((log, idx) => (
                  <div key={idx} style={{ marginBottom: 6, display: 'flex', gap: 8 }}>
                    <span style={{ color: '#666', flexShrink: 0 }}>{log.timestamp}</span>
                    <span
                      style={{
                        color: logTypeColors[log.type] || '#d4d4d4',
                        background: `${logTypeColors[log.type]}20` || 'transparent',
                        padding: '0 4px',
                        borderRadius: 2,
                        flexShrink: 0,
                        minWidth: 50,
                      }}
                    >
                      {logTypeLabels[log.type] || log.type}
                    </span>
                    <span style={{ wordBreak: 'break-all', whiteSpace: 'pre-wrap' }}>{log.content}</span>
                  </div>
                ))}
                <div ref={logsEndRef} />
              </>
            )}
          </div>

          {/* Final Result */}
          {executionResult !== null && executionResult !== '' && (
            <div style={{ marginTop: 12 }}>
              <div style={{ fontSize: 12, color: '#999', marginBottom: 4 }}>最终结果:</div>
              <div style={{
                background: executionSuccess ? '#f6ffed' : '#fff2f0',
                border: `1px solid ${executionSuccess ? '#b7eb8f' : '#ffccc7'}`,
                padding: 12,
                borderRadius: 4,
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

      {/* Execution History */}
      <div>
        <h4 style={{ marginBottom: 12 }}>执行历史</h4>
        {records.length === 0 ? (
          <Empty description="暂无执行记录" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        ) : (
          records.map(record => (
            <div
              key={record.id}
              style={{
                border: '1px solid #f0f0f0',
                borderRadius: 8,
                padding: 12,
                marginBottom: 12,
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <span style={{ fontSize: 12, color: '#999' }}>
                    {new Date(record.started_at).toLocaleString()}
                  </span>
                  {record.executor && (
                    <Tag color={record.executor === 'claudecode' ? 'purple' : 'cyan'}>
                      {record.executor === 'claudecode' ? 'Claude Code' : 'JoinAI'}
                    </Tag>
                  )}
                  {record.model && (
                    <Tag color='blue'>
                      {record.model}
                    </Tag>
                  )}
                  {record.usage?.duration_ms && (
                    <span style={{ fontSize: 11, color: '#52c41a', fontWeight: 500 }}>
                      {(record.usage.duration_ms / 1000).toFixed(2)}s
                    </span>
                  )}
                </div>
                <span style={{
                  fontSize: 12,
                  padding: '2px 8px',
                  borderRadius: 10,
                  backgroundColor: record.status === 'success' ? '#52c41a' : record.status === 'failed' ? '#ff4d4f' : '#1890ff',
                  color: '#fff',
                }}>
                  {record.status === 'success' ? '成功' : record.status === 'failed' ? '失败' : '进行中'}
                </span>
              </div>
              <div style={{ fontSize: 12, color: '#666', marginBottom: 8 }}>
                命令: <code>{record.command}</code>
              </div>

              {/* Result */}
              {record.result !== null && record.result !== '' && (
                <div style={{ marginBottom: 8 }}>
                  <div style={{ fontSize: 12, color: '#999', marginBottom: 4 }}>执行结果:</div>
                  <div style={{
                    background: record.status === 'success' ? '#f6ffed' : '#fff2f0',
                    border: `1px solid ${record.status === 'success' ? '#b7eb8f' : '#ffccc7'}`,
                    padding: 12,
                    borderRadius: 4,
                    fontSize: 13,
                    color: '#333',
                    whiteSpace: 'pre-wrap',
                    wordBreak: 'break-all',
                  }}>
                    {record.result}
                  </div>
                </div>
              )}

              {/* Usage */}
              {record.usage && (
                <div style={{ fontSize: 11, color: '#999', marginBottom: 8 }}>
                  <span style={{ marginRight: 12 }}>Input: {record.usage.input_tokens.toLocaleString()}</span>
                  <span style={{ marginRight: 12 }}>Output: {record.usage.output_tokens.toLocaleString()}</span>
                  {record.usage.total_cost_usd !== null && (
                    <span style={{ color: '#faad14' }}>${record.usage.total_cost_usd.toFixed(6)}</span>
                  )}
                </div>
              )}

              {/* Logs */}
              {record.logs && record.logs !== '[]' && (
                <details>
                  <summary style={{ cursor: 'pointer', color: '#1890ff' }}>
                    查看日志 ({JSON.parse(record.logs).length} 条)
                  </summary>
                  <div style={{
                    background: '#1e1e1e',
                    color: '#d4d4d4',
                    padding: 8,
                    borderRadius: 4,
                    fontFamily: 'monospace',
                    fontSize: 11,
                    marginTop: 8,
                    maxHeight: 300,
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
  );
}
