import { useEffect, useState } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty, Input, App, Popconfirm, Tag, Badge, Pagination } from 'antd';
import { PlayCircleOutlined, EditOutlined, DeleteOutlined, SettingOutlined, CheckCircleOutlined, ReloadOutlined, CopyOutlined, ArrowLeftOutlined, StopOutlined } from '@ant-design/icons';
import { StatusPicker } from './StatusPicker';
import { TagCheckCardGroup } from './TagCheckCard';
import { PieChart, PieChartLegend } from './PieChart';
import { TodoSettingsModal } from './TodoSettingsModal';
import * as db from '../utils/database';
import { formatLocalDateTime } from '../utils/datetime';
import { AnimatedNumber } from './AnimatedNumber';
import { getExecutorOption } from '../types';
import XMarkdown from '@ant-design/x-markdown';
import type { ExecutionSummary, Todo } from '../types';

const { TextArea } = Input;

export function TodoDetail() {
  const { state, dispatch } = useApp();
  const { message } = App.useApp();
  const { todos, selectedTodoId, executionRecords, runningTasks } = state;
  const [isMobile, setIsMobile] = useState(false);
  const selectedTodo = todos.find(t => t.id === selectedTodoId);

  useEffect(() => {
    const checkMobile = () => setIsMobile(window.innerWidth < 768);
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState('');
  const [editPrompt, setEditPrompt] = useState('');
  const [editStatus, setEditStatus] = useState<string>('pending');
  const [editTags, setEditTags] = useState<number[]>([]);
  const [summary, setSummary] = useState<ExecutionSummary | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Execution history pagination state
  const [historyPage, setHistoryPage] = useState(1);
  const [historyLimit, setHistoryLimit] = useState(5);
  const [historyTotal, setHistoryTotal] = useState(0);

  const records = selectedTodoId ? executionRecords[selectedTodoId] || [] : [];

  // Check if current todo is running in the global panel
  const currentRunningTask = Object.values(runningTasks).find(
    t => t.todoId === selectedTodoId
  );
  const isExecuting = !!currentRunningTask && currentRunningTask.status === 'running';

  const loadExecutionRecords = async (page = 1, limit = historyLimit) => {
    if (!selectedTodo) return;
    const pageData = await db.getExecutionRecords(selectedTodo.id, page, limit);
    dispatch({
      type: 'SET_EXECUTION_RECORDS',
      payload: { todoId: selectedTodo.id, records: pageData.records }
    });
    setHistoryPage(pageData.page);
    setHistoryLimit(pageData.limit);
    setHistoryTotal(pageData.total);
  };

  useEffect(() => {
    if (selectedTodo) {
      setEditTitle(selectedTodo.title);
      setEditPrompt(selectedTodo.prompt || '');
      setEditStatus(selectedTodo.status);
      setEditTags((selectedTodo as any).tag_ids || []);
      setHistoryPage(1);

      loadExecutionRecords(1, historyLimit);

      db.getExecutionSummary(selectedTodo.id).then(sum => {
        setSummary(sum);
      });
    } else {
      setIsEditing(false);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedTodoId, selectedTodo, dispatch]);

  const handleExecute = async () => {
    if (!selectedTodo) return;
    try {
      await db.executeTodo(
        selectedTodo.id,
        selectedTodo.prompt || selectedTodo.title,
        selectedTodo.executor || undefined
      );
      message.success('任务已开始执行');
    } catch (error) {
      message.error('执行失败: ' + error);
    }
  };

  const handleStopExecution = async () => {
    if (currentRunningTask) {
      try {
        await db.stopExecution(currentRunningTask.taskId);
        message.info('已发送停止指令');
      } catch (error) {
        message.error('停止失败: ' + error);
      }
    }
  };

  const handleStatusChange = async (newStatus: string) => {
    if (!selectedTodo) return;
    const updated = await db.updateTodo(selectedTodo.id, selectedTodo.title, selectedTodo.prompt || '', newStatus);
    dispatch({ type: 'UPDATE_TODO', payload: updated });
    message.success('状态已更新');
  };

  const handleSaveEdit = async () => {
    if (!selectedTodo) return;
    const updated = await db.updateTodo(
      selectedTodo.id,
      editTitle,
      editPrompt,
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
  const executorOpt = getExecutorOption(executor);

  return (
    <div className="detail-panel">
      {/* Mobile Back Button */}
      {isMobile && (
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => {
            dispatch({ type: 'SELECT_TODO', payload: null });
          }}
          style={{ marginBottom: 8, marginLeft: -4 }}
        >
          返回
        </Button>
      )}
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
              value={editPrompt}
              onChange={e => setEditPrompt(e.target.value)}
              rows={3}
              placeholder="输入 Prompt..."
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
                {selectedTodo.prompt && (
                  <p className="card-description">{selectedTodo.prompt}</p>
                )}
                {/* Info tags: executor + scheduler */}
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap', marginTop: 8 }}>
                  <Tag color={executorOpt.color} style={{ fontWeight: 600 }}>
                    {executorOpt.icon} {executorOpt.label}
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
                {/* Running status indicator */}
                {isExecuting && (
                  <div style={{ marginTop: 8, display: 'flex', alignItems: 'center', gap: 6 }}>
                    <Badge status="processing" />
                    <span style={{ fontSize: 13, color: 'var(--color-primary)', fontWeight: 500 }}>
                      正在执行中...（查看底部面板）
                    </span>
                  </div>
                )}
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
                        <AnimatedNumber value={totalTokens} duration={1.2} chineseFormat />
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
                      <AnimatedNumber value={summary.total_executions} duration={0.8} />
                    </strong>{' '}
                    次
                  </span>
                  <span style={{ color: 'var(--color-border)' }}>|</span>
                  <span style={{ color: 'var(--color-success)' }}>
                    成功 <AnimatedNumber value={summary.success_count} duration={0.8} />
                  </span>
                  <span style={{ color: 'var(--color-error)' }}>
                    失败 <AnimatedNumber value={summary.failed_count} duration={0.8} />
                  </span>
                  {summary.total_cost_usd !== null &&
                    summary.total_cost_usd !== undefined && (
                      <>
                        <span style={{ color: 'var(--color-border)' }}>|</span>
                        <span style={{ color: 'var(--color-warning)', fontWeight: 600 }}>
                          $<AnimatedNumber value={summary.total_cost_usd} duration={0.8} decimals={4} />
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
              icon={<PlayCircleOutlined />}
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
              block
              className="btn-execute"
            >
              执行任务
            </Button>
          )}
        </div>
      )}

      {/* Execution History */}
      <div style={{ paddingBottom: 20, flexShrink: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
          <h4 style={{ margin: 0, fontSize: 15, fontWeight: 700, color: 'var(--color-text)' }}>执行历史</h4>
          <Button
            type="text"
            size="small"
            icon={<ReloadOutlined />}
            onClick={() => loadExecutionRecords(historyPage, historyLimit)}
            loading={isExecuting}
          >
            刷新
          </Button>
        </div>
        {records.length === 0 ? (
          <Empty description="暂无执行记录" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        ) : (
          <>
            {records.map(record => (
              <div
                key={record.id}
                className={`history-card history-card-${record.status}`}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8, flexWrap: 'wrap', gap: 8 }}>
                  <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
                    <span style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                      {formatLocalDateTime(record.started_at)}
                    </span>
                    {record.executor && (() => {
                      const recOpt = getExecutorOption(record.executor);
                      return (
                        <Tag color={recOpt.color} style={{ fontWeight: 600 }}>
                          {recOpt.icon} {recOpt.label}
                        </Tag>
                      );
                    })()}
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
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
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
                    {record.status === 'running' && (() => {
                      const taskId = currentRunningTask?.taskId || selectedTodo?.task_id;
                      if (!taskId) return null;
                      return (
                        <Popconfirm title="确定强制停止该任务？" onConfirm={async () => {
                          try {
                            await db.stopExecution(taskId);
                            message.info('已发送停止指令');
                          } catch (error) {
                            message.error('停止失败: ' + error);
                          }
                        }} okText="停止" cancelText="取消">
                          <Button type="text" danger size="small" icon={<StopOutlined />} style={{ fontSize: 12 }}>
                            停止
                          </Button>
                        </Popconfirm>
                      );
                    })()}
                  </div>
                </div>
                {record.command && (
                  <div style={{ fontSize: 11, color: 'var(--color-text-quaternary)', marginBottom: 8, fontFamily: 'var(--font-mono)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {record.command}
                  </div>
                )}

                {record.result !== null && record.result !== '' && (
                  <div className={`history-result ${record.status === 'success' ? 'history-result-success' : 'history-result-failed'}`}>
                    <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: 4 }}>
                      <Button
                        type="text"
                        size="small"
                        icon={<CopyOutlined />}
                        onClick={async () => {
                          try {
                            await navigator.clipboard.writeText(record.result || '');
                            message.success('已复制到剪贴板');
                          } catch {
                            message.error('复制失败');
                          }
                        }}
                      />
                    </div>
                    <XMarkdown content={record.result} />
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

                {(() => {
                  const isRunning = record.status === 'running';
                  const liveLogs = isRunning && currentRunningTask ? currentRunningTask.logs : null;
                  const restLogs: Array<{ timestamp?: string; type?: string; content?: string }> = (() => {
                    try { return record.logs && record.logs !== '[]' ? JSON.parse(record.logs) : []; }
                    catch { return []; }
                  })();
                  const displayLogs = liveLogs && liveLogs.length > 0 ? liveLogs : restLogs;

                  if (!isRunning && displayLogs.length === 0) return null;

                  return (
                    <details style={{ marginTop: 8 }} open={isRunning}>
                      <summary style={{ cursor: 'pointer', color: 'var(--color-primary)', fontSize: 12, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 }}>
                        <span>查看日志 ({displayLogs.length} 条){isRunning && liveLogs && liveLogs.length > 0 ? ' · 实时' : ''}</span>
                        <ReloadOutlined
                          style={{ fontSize: 11 }}
                          onClick={(e) => { e.preventDefault(); e.stopPropagation(); loadExecutionRecords(historyPage, historyLimit); }}
                        />
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
                        {displayLogs.length === 0 ? (
                          <div style={{ color: '#64748b' }}>等待输出...</div>
                        ) : (
                          displayLogs.map((log, idx) => (
                            <div key={idx} style={{ marginBottom: 4, display: 'flex', gap: 8 }}>
                              <span style={{ color: '#64748b', flexShrink: 0 }}>{log.timestamp}</span>
                              <span style={{ color: logTypeColors[log.type || ''] || '#cbd5e1' }}>
                                [{logTypeLabels[log.type || ''] || log.type}]
                              </span>
                              <span>{log.content}</span>
                            </div>
                          ))
                        )}
                      </div>
                    </details>
                  );
                })()}
              </div>
            ))}
            {historyTotal > historyLimit && (
              <div style={{ display: 'flex', justifyContent: 'center', marginTop: 16 }}>
                <Pagination
                  current={historyPage}
                  pageSize={historyLimit}
                  total={historyTotal}
                  onChange={(page, pageSize) => {
                    if (pageSize !== historyLimit) {
                      setHistoryLimit(pageSize);
                      loadExecutionRecords(1, pageSize);
                    } else {
                      loadExecutionRecords(page, historyLimit);
                    }
                  }}
                  size="small"
                  showSizeChanger
                  pageSizeOptions={['5', '10', '20']}
                />
              </div>
            )}
          </>
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
