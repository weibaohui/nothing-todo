import { useEffect, useState } from 'react';
import { useApp } from '../hooks/useApp';
import { Button, Empty, App, Popconfirm, Tag, Badge, Pagination, Segmented, Modal, Input } from 'antd';
import { PlayCircleOutlined, EditOutlined, DeleteOutlined, SettingOutlined, CheckCircleOutlined, ReloadOutlined, CopyOutlined, ArrowLeftOutlined, StopOutlined, DownOutlined, UpOutlined, UnorderedListOutlined, MessageOutlined, FileTextOutlined } from '@ant-design/icons';
import { StatusPicker } from './StatusPicker';
import { PieChart } from './PieChart';
import { TodoSettingsDrawer } from './TodoSettingsDrawer';
import { TodoEditDrawer } from './TodoEditDrawer';
import { ChatView } from './ChatView';
import { parseLogsToMessages } from './ChatView';
import * as db from '../utils/database';
import { formatLocalDateTime } from '../utils/datetime';
import { conversationToYaml } from '../utils/markdown';
import { AnimatedNumber } from './AnimatedNumber';
import { getExecutorOption, supportsResume } from '../types';
import XMarkdown from '@ant-design/x-markdown';
import type { ExecutionSummary, Todo, TodoItem, ExecutionRecord, LogEntry } from '../types';

/** 可展开的 Prompt 内容展示组件 */
function PromptDisplay({ content }: { content: string }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div style={{ marginTop: 8 }}>
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 4,
          fontSize: 12,
          color: 'var(--color-text-secondary)',
          cursor: 'pointer',
          userSelect: 'none',
        }}
      >
        <span>{expanded ? '▼' : '▶'}</span>
        <span>Prompt</span>
      </div>
      {expanded && (
        <div
          style={{
            marginTop: 6,
            padding: '8px 12px',
            borderRadius: 8,
            background: 'var(--color-bg-elevated)',
            border: '1px solid var(--color-border-light)',
            maxHeight: 300,
            overflow: 'auto',
          }}
        >
          <XMarkdown content={content} />
        </div>
      )}
    </div>
  );
}

/** 内联 Token 统计摘要，支持展开查看详细分项 */
function InlineTokenStats({ input, output, cacheRead, cacheCreate, totalTokens, summary }: {
  input: number; output: number; cacheRead: number; cacheCreate: number; totalTokens: number; summary: ExecutionSummary;
}) {
  const [expanded, setExpanded] = useState(false);
  // 推理输入 = 输入 + 缓存读 + 缓存写
  const reasoningInput = input + cacheRead + cacheCreate;
  // 成本输入 = 输入 + 缓存写
  const costInput = input + cacheCreate;
  // 输出率 = 输出 / 成本输入 * 100%
  const outputRate = costInput > 0 ? (output / costInput * 100) : 0;

  const tokenSegments = [
    { value: input, color: '#3b82f6', label: '输入' },
    { value: output, color: '#22c55e', label: '输出' },
    { value: cacheRead, color: '#f59e0b', label: '缓存读' },
    { value: cacheCreate, color: '#a78bfa', label: '缓存写' },
  ];
  const extraSegments = [
    { value: reasoningInput, color: '#ec4899', label: '推理输入' },
    { value: costInput, color: '#f97316', label: '成本输入' },
    { value: outputRate, color: '#14b8a6', label: '输出率', isPercent: true },
  ];
  return (
    <div style={{ position: 'relative', display: 'inline-flex', alignItems: 'center' }}>
      <button
        type="button"
        aria-expanded={expanded}
        aria-label="Token 统计摘要，点击展开详情"
        onClick={() => setExpanded(!expanded)}
        style={{ display: 'inline-flex', alignItems: 'center', gap: 8, cursor: 'pointer', userSelect: 'none', fontSize: 11, color: 'var(--color-text-secondary)', background: 'none', border: 'none', padding: 0 }}
      >
        <PieChart segments={tokenSegments.filter(s => s.value > 0)} size={20} />
        <span style={{ fontWeight: 700, color: 'var(--color-text)', fontSize: 13 }}><AnimatedNumber value={totalTokens} duration={1.2} chineseFormat /></span>
        <span>Tokens</span>
        <span style={{ color: 'var(--color-border)' }}>|</span>
        <span>执行 <strong style={{ color: 'var(--color-text)' }}>{summary.total_executions}</strong> 次</span>
        <span style={{ color: 'var(--color-success)' }}>成功 {summary.success_count}</span>
        <span style={{ color: 'var(--color-error)' }}>失败 {summary.failed_count}</span>
        {summary.total_cost_usd != null && (
          <span style={{ color: 'var(--color-warning)', fontWeight: 600 }}>${summary.total_cost_usd.toFixed(4)}</span>
        )}
        {expanded ? <UpOutlined style={{ fontSize: 10 }} /> : <DownOutlined style={{ fontSize: 10 }} />}
      </button>
      {expanded && (
        <div style={{ position: 'absolute', top: '100%', left: 0, zIndex: 10, marginTop: 4, background: '#fff', border: '1px solid var(--color-border-light)', borderRadius: 8, padding: 10, boxShadow: '0 4px 12px rgba(0,0,0,0.1)', minWidth: 280 }}>
          <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', fontSize: 11 }}>
            {tokenSegments.filter(s => s.value > 0).map(s => (
              <span key={s.label} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <span style={{ width: 8, height: 8, borderRadius: '50%', background: s.color }} />
                {s.label}: <AnimatedNumber value={s.value} duration={1.2} chineseFormat />
              </span>
            ))}
          </div>
          <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', fontSize: 11, marginTop: 8, paddingTop: 8, borderTop: '1px solid var(--color-border-light)' }}>
            {extraSegments.map(s => (
              <span key={s.label} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <span style={{ width: 8, height: 8, borderRadius: '50%', background: s.color }} />
                {s.label}: {s.isPercent ? s.value.toFixed(1) + '%' : <AnimatedNumber value={s.value} duration={1.2} chineseFormat />}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/** 任务进度展示组件，显示子项完成情况 */
function ProgressWidget({ items }: { items: TodoItem[] }) {
  const [expanded, setExpanded] = useState(false);
  const total = items.length;
  const completed = items.filter(t => t.status === 'completed').length;
  const pct = Math.round((completed / total) * 100);

  return (
    <div style={{ position: 'relative', flexShrink: 0 }}>
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          background: 'var(--color-bg-elevated)',
          borderRadius: 6,
          padding: '4px 10px',
          border: `1px solid ${expanded ? 'var(--color-primary)' : 'var(--color-border-light)'}`,
          minWidth: 120,
          cursor: 'pointer',
          userSelect: 'none',
          transition: 'border-color 0.2s',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 3 }}>
          <span style={{ fontSize: 10, fontWeight: 600, color: 'var(--color-text-secondary)' }}>进度</span>
          <span style={{ fontSize: 10, color: 'var(--color-primary)', fontWeight: 600 }}>{completed}/{total} ({pct}%)</span>
        </div>
        <div style={{ height: 3, borderRadius: 2, background: 'var(--color-border-light)', marginBottom: 3 }}>
          <div style={{ height: '100%', borderRadius: 2, background: 'var(--color-primary)', width: `${pct}%`, transition: 'width 0.3s' }} />
        </div>
        <div style={{ display: 'flex', gap: 3, flexWrap: 'wrap' }}>
          {items.map((item, idx) => (
            <span key={item.id || idx} style={{ fontSize: 10, lineHeight: '14px', color: item.status === 'completed' ? 'var(--color-text-tertiary)' : item.status === 'in_progress' ? 'var(--color-primary)' : 'var(--color-text-secondary)', textDecoration: item.status === 'completed' ? 'line-through' : 'none', maxWidth: 80, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {item.status === 'completed' ? '✓' : item.status === 'in_progress' ? '●' : '○'} {item.content}
            </span>
          ))}
        </div>
      </div>
      {expanded && (
        <div style={{
          position: 'absolute',
          top: '100%',
          right: 0,
          zIndex: 20,
          marginTop: 4,
          background: '#fff',
          border: '1px solid var(--color-border-light)',
          borderRadius: 8,
          padding: 12,
          boxShadow: '0 6px 20px rgba(0,0,0,0.12)',
          minWidth: 260,
          maxWidth: 360,
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
            <span style={{ fontSize: 12, fontWeight: 700, color: 'var(--color-text)' }}>任务进度</span>
            <span style={{ fontSize: 11, color: 'var(--color-primary)', fontWeight: 600 }}>{completed}/{total} ({pct}%)</span>
          </div>
          <div style={{ height: 4, borderRadius: 2, background: 'var(--color-border-light)', marginBottom: 10 }}>
            <div style={{ height: '100%', borderRadius: 2, background: 'var(--color-primary)', width: `${pct}%`, transition: 'width 0.3s' }} />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6, maxHeight: 280, overflow: 'auto' }}>
            {items.map((item, idx) => (
              <div key={item.id || idx} style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: 8,
                fontSize: 12,
                lineHeight: '18px',
                color: item.status === 'completed' ? 'var(--color-text-tertiary)' : item.status === 'in_progress' ? 'var(--color-primary)' : 'var(--color-text-secondary)',
                textDecoration: item.status === 'completed' ? 'line-through' : 'none',
                padding: '4px 8px',
                borderRadius: 4,
                background: item.status === 'in_progress' ? 'var(--color-primary-bg)' : 'transparent',
              }}>
                <span style={{ flexShrink: 0, marginTop: 2 }}>
                  {item.status === 'completed' ? '✓' : item.status === 'in_progress' ? '●' : '○'}
                </span>
                <span style={{ wordBreak: 'break-word' }}>{item.content}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/** 任务详情面板，包含执行、编辑、历史记录等功能 */
export function TodoDetail() {
  const { state, dispatch } = useApp();
  const { message } = App.useApp();
  const { todos, selectedTodoId, executionRecords, runningTasks } = state;
  const [isMobile, setIsMobile] = useState(false);
  const [isWide, setIsWide] = useState(false);
  const [selectedHistoryRecordId, setSelectedHistoryRecordId] = useState<number | null>(null);
  const [viewMode, setViewMode] = useState<'log' | 'chat'>('log');
  const selectedTodo = todos.find(t => t.id === selectedTodoId);

  useEffect(() => {
    const checkMobile = () => setIsMobile(window.innerWidth < 768);
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  useEffect(() => {
    const checkWide = () => setIsWide(window.innerWidth >= 1440);
    checkWide();
    window.addEventListener('resize', checkWide);
    return () => window.removeEventListener('resize', checkWide);
  }, []);

  const [isEditing, setIsEditing] = useState(false);
  const [summary, setSummary] = useState<ExecutionSummary | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Execution history pagination state
  const [historyPage, setHistoryPage] = useState(1);
  const [historyLimit, setHistoryLimit] = useState(5);
  const [historyTotal, setHistoryTotal] = useState(0);

  const records = selectedTodoId ? executionRecords[selectedTodoId] || [] : [];
  const selectedHistoryRecord = selectedHistoryRecordId
    ? records.find(r => r.id === selectedHistoryRecordId) || null
    : null;

  // Check if current todo is executing (has any running task)
  const isExecuting = Object.values(runningTasks).some(
    t => t.todoId === selectedTodoId && t.status === 'running'
  );

  // Find the running task that matches a specific execution record by task_id
  const getRunningTaskForRecord = (record: ExecutionRecord) => {
    if (record.task_id) {
      return runningTasks[record.task_id] || null;
    }
    // Fallback: match by todoId for records without task_id
    return Object.values(runningTasks).find(t => t.todoId === record.todo_id) || null;
  };

  // Helper to resolve execution stats from record or running task
  const resolveExecutionStats = (record: ExecutionRecord, isRunning: boolean) => {
    if (isRunning) {
      const task = getRunningTaskForRecord(record);
      if (task?.executionStats) return task.executionStats;
    }
    return record.execution_stats;
  };

  const loadExecutionRecords = async (page = 1, limit = historyLimit) => {
    if (!selectedTodo) return;
    try {
      const pageData = await db.getExecutionRecords(selectedTodo.id, page, limit);
      dispatch({
        type: 'SET_EXECUTION_RECORDS',
        payload: { todoId: selectedTodo.id, records: pageData.records }
      });
      setHistoryPage(pageData.page);
      setHistoryLimit(pageData.limit);
      setHistoryTotal(pageData.total);
    } catch {
      // ignore: interceptor already shows error
    }
  };

  const refreshSingleRecord = async (recordId: number) => {
    if (!selectedTodo) return;
    try {
      const record = await db.getExecutionRecord(recordId);
      dispatch({
        type: 'UPDATE_EXECUTION_RECORD',
        payload: { todoId: selectedTodo.id, record }
      });
    } catch {
      // ignore
    }
  };

  useEffect(() => {
    let cancelled = false;
    if (selectedTodo) {
      setHistoryPage(1);

      db.getExecutionRecords(selectedTodo.id, 1, historyLimit).then(pageData => {
        if (cancelled) return;
        dispatch({
          type: 'SET_EXECUTION_RECORDS',
          payload: { todoId: selectedTodo.id, records: pageData.records }
        });
        setHistoryPage(pageData.page);
        setHistoryLimit(pageData.limit);
        setHistoryTotal(pageData.total);
      }).catch(() => {});

      db.getExecutionSummary(selectedTodo.id).then(sum => {
        if (!cancelled) setSummary(sum);
      }).catch(() => {});
    } else {
      setIsEditing(false);
    }
    return () => { cancelled = true; };
  }, [selectedTodoId, selectedTodo, dispatch, historyLimit]);

  useEffect(() => {
    setSelectedHistoryRecordId(null);
  }, [selectedTodoId]);

  useEffect(() => {
    if (!isWide || records.length === 0) return;
    if (selectedHistoryRecordId !== null && records.find(r => r.id === selectedHistoryRecordId)) return;
    setSelectedHistoryRecordId(records[0].id);
  }, [isWide, records, selectedHistoryRecordId]);

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
      message.error('执行失败: ' + (error instanceof Error ? error.message : String(error)));
    }
  };

  const handleStopExecution = async (recordId: number) => {
    try {
      await db.stopExecution(recordId);
      message.info('已发送停止指令');
      await loadExecutionRecords(historyPage, historyLimit);
    } catch (error) {
      message.error('停止失败: ' + (error instanceof Error ? error.message : String(error)));
    }
  };

  // Resume conversation state & handlers
  const [resumeModalOpen, setResumeModalOpen] = useState(false);
  const [resumeRecordId, setResumeRecordId] = useState<number | null>(null);
  const [resumeMessage, setResumeMessage] = useState('');
  const [resumeLoading, setResumeLoading] = useState(false);

  const handleOpenResume = (record: ExecutionRecord) => {
    setResumeRecordId(record.id);
    setResumeMessage(selectedTodo?.prompt || selectedTodo?.title || '');
    setResumeModalOpen(true);
  };

  const handleResumeConfirm = async () => {
    if (!resumeRecordId) return;
    setResumeLoading(true);
    try {
      await db.resumeExecutionRecord(resumeRecordId, resumeMessage);
      message.success('已继续对话，任务开始执行');
      setResumeModalOpen(false);
      await loadExecutionRecords(historyPage, historyLimit);
    } catch (error) {
      message.error('继续对话失败: ' + (error instanceof Error ? error.message : String(error)));
    } finally {
      setResumeLoading(false);
    }
  };

  const parseRecordLogs = (record: ExecutionRecord): LogEntry[] => {
    try {
      return record.logs && record.logs !== '[]' ? JSON.parse(record.logs) : [];
    } catch {
      return [];
    }
  };

  const hasLogs = (record: ExecutionRecord): boolean => {
    return !!record.logs && record.logs !== '[]';
  };

  const handleExportMarkdown = (record: ExecutionRecord) => {
    const logs = parseRecordLogs(record);
    const messages = parseLogsToMessages(logs);
    const executorLabel = record.executor ? getExecutorOption(record.executor).label : undefined;
    const content = conversationToYaml(messages, {
      title: selectedTodo?.title,
      executor: executorLabel,
      model: record.model || undefined,
      startedAt: record.started_at,
      status: record.status,
    });
    const blob = new Blob([content], { type: 'application/x-yaml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    a.download = `exec-${record.id}-${timestamp}.yaml`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    message.success('导出成功');
  };

  const handleStatusChange = async (newStatus: string) => {
    if (!selectedTodo) return;
    try {
      const updated = await db.updateTodo(selectedTodo.id, selectedTodo.title, selectedTodo.prompt || '', newStatus);
      dispatch({ type: 'UPDATE_TODO', payload: updated });
      message.success('状态已更新');
    } catch {
      // ignore: interceptor already shows error
    }
  };

  const handleSaveEdit = async (editTitle: string, editPrompt: string) => {
    if (!selectedTodo) return;
    try {
      const updated = await db.updateTodo(
        selectedTodo.id,
        editTitle,
        editPrompt,
        selectedTodo.status,
      );
      dispatch({
        type: 'UPDATE_TODO',
        payload: updated as Todo
      });
    } catch {
      // ignore: interceptor already shows error
    }
  };

  const handleDelete = async () => {
    if (!selectedTodo) return;
    try {
      await db.deleteTodo(selectedTodo.id);
      dispatch({ type: 'DELETE_TODO', payload: selectedTodo.id });
      dispatch({ type: 'SELECT_TODO', payload: null });
      message.success('删除成功');
    } catch {
      // ignore: interceptor already shows error
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
  const executorOpt = getExecutorOption(executor);

  // Resolve current todo progress for header widget — follows selected execution record
  const currentTodoProgress = (() => {
    // Try to find the record by selectedHistoryRecordId first, then fallback to first record
    const source = selectedHistoryRecord
      || (selectedHistoryRecordId ? records.find(r => r.id === selectedHistoryRecordId) : null)
      || (records.length > 0 ? records[0] : null);
    if (!source) return null;
    if (source.status === 'running') {
      const task = getRunningTaskForRecord(source);
      if (task?.todoProgress?.length) return task.todoProgress;
    }
    if (source.todo_progress) {
      try {
        const parsed = JSON.parse(source.todo_progress);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed;
      } catch { /* ignore */ }
    }
    return null;
  })();

  return (
    <div className={`detail-panel${isWide ? ' detail-panel-wide' : ''}`}>
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
      {/* Unified Header: Title + Stats + Execute */}
      <div className="detail-card header-card">
        {/* Row 1: Title + Action Buttons */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
          <StatusPicker value={selectedTodo.status} onChange={handleStatusChange} disabled={isExecuting} />
          <h2 className="card-title" style={{ margin: 0, flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{selectedTodo.title}</h2>
          <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
            <Button type="text" icon={<SettingOutlined />} onClick={() => setSettingsOpen(true)} className="icon-btn" aria-label="任务设置" />
            <Button type="text" icon={<EditOutlined />} onClick={() => setIsEditing(true)} className="icon-btn" aria-label="编辑任务" />
            <Popconfirm title="删除任务" description="确定要删除吗？" onConfirm={handleDelete}>
              <Button type="text" danger icon={<DeleteOutlined />} className="icon-btn" aria-label="删除任务" />
            </Popconfirm>
          </div>
        </div>
        {/* Row 2: Tags + Inline Token Stats + Progress Widget */}
        <div style={{ display: 'flex', alignItems: 'flex-start', gap: 10, flexWrap: 'wrap' }}>
          {/* Tags & Meta */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <Tag color={executorOpt.color} style={{ fontWeight: 600, fontSize: 11 }}>
              {executorOpt.icon} {executorOpt.label}
            </Tag>
            {selectedTodo.scheduler_enabled ? (
              <Tag color="var(--color-primary)" style={{ fontWeight: 600, fontSize: 11 }}>
                调度: {selectedTodo.scheduler_config}
              </Tag>
            ) : (
              <Tag style={{ fontWeight: 600, fontSize: 11, color: 'var(--color-text-tertiary)', borderColor: 'var(--color-border)' }}>
                调度: 关闭
              </Tag>
            )}
            {records.length > 0 && (
              <span style={{ fontSize: 11, color: 'var(--color-text-tertiary)' }}>
                上次: {formatLocalDateTime(records[0].started_at)}
              </span>
            )}
            {selectedTodo.scheduler_next_run_at && (
              <span style={{ fontSize: 11, color: 'var(--color-success)' }}>
                下次: {formatLocalDateTime(selectedTodo.scheduler_next_run_at)}
              </span>
            )}
            {isExecuting && (
              <>
                <span style={{ color: 'var(--color-border)' }}>|</span>
                <Badge status="processing" />
                <span style={{ fontSize: 12, color: 'var(--color-primary)', fontWeight: 500 }}>执行中...</span>
              </>
            )}
          </div>
          {/* Inline Token Stats */}
          {summary && summary.total_executions > 0 && (() => {
            const input = summary.total_input_tokens;
            const output = summary.total_output_tokens;
            const cacheRead = (summary as any).total_cache_read_tokens ?? 0;
            const cacheCreate = (summary as any).total_cache_creation_tokens ?? 0;
            const totalTokens = input + output + cacheRead + cacheCreate;
            return (
              <InlineTokenStats input={input} output={output} cacheRead={cacheRead} cacheCreate={cacheCreate} totalTokens={totalTokens} summary={summary} />
            );
          })()}
          {/* Progress Widget (rightmost) */}
          {currentTodoProgress && (
            <div style={{ marginLeft: 'auto', flexShrink: 0 }}>
              <ProgressWidget items={currentTodoProgress} />
            </div>
          )}
        </div>
        {selectedTodo.prompt && <PromptDisplay content={selectedTodo.prompt} />}
        {/* Execute Button Row */}
        <Button
          type="primary"
          icon={<PlayCircleOutlined />}
          onClick={handleExecute}
          block
          className="btn-execute btn-execute-compact"
        >
          执行任务
        </Button>
      </div>

      {/* Execution History */}
      <div
        style={isWide
          ? { flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', minHeight: 0 }
          : { paddingBottom: 20, flexShrink: 0 }
        }
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12, ...(isWide ? { flexShrink: 0 } : {}) }}>
          <h4 style={{ margin: 0, fontSize: 15, fontWeight: 700, color: 'var(--color-text)' }}>执行历史</h4>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Segmented
              size="small"
              value={viewMode}
              onChange={(value) => setViewMode(value as 'log' | 'chat')}
              options={[
                { value: 'log', icon: <UnorderedListOutlined />, label: '日志' },
                { value: 'chat', icon: <MessageOutlined />, label: '对话' },
              ]}
            />
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
        </div>
        {records.length === 0 ? (
          <Empty description="暂无执行记录" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        ) : isWide ? (
          <div style={{ flex: 1, display: 'flex', gap: 16, overflow: 'hidden', minHeight: 0 }}>
            {/* Left: History List */}
            <div style={{ width: 320, flexShrink: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
              <div className="history-list-column">
                {records.map(record => {
                  const isSelected = selectedHistoryRecordId === record.id;
                  const recExecutor = record.executor ? getExecutorOption(record.executor) : null;
                  return (
                    <div
                      key={record.id}
                      className={`history-item-compact${isSelected ? ' selected' : ''}${record.status === 'failed' ? ' failed' : record.status === 'running' ? ' running' : ''}`}
                      onClick={() => setSelectedHistoryRecordId(record.id)}
                    >
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
                        <span style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                          {formatLocalDateTime(record.started_at)}
                        </span>
                        <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                          {record.status !== 'running' && supportsResume(record) && (
                            <MessageOutlined
                              style={{ fontSize: 12, color: 'var(--color-primary)', cursor: 'pointer' }}
                              title="继续对话"
                              onClick={(e) => { e.stopPropagation(); handleOpenResume(record); }}
                            />
                          )}
                          {hasLogs(record) && (
                            <FileTextOutlined
                              style={{ fontSize: 12, color: 'var(--color-text-tertiary)', cursor: 'pointer' }}
                              title="导出为YAML"
                              onClick={(e) => { e.stopPropagation(); handleExportMarkdown(record); }}
                            />
                          )}
                          <span style={{
                            fontSize: 10,
                            padding: '2px 8px',
                            borderRadius: 10,
                            backgroundColor: record.status === 'success' ? 'var(--color-success)' : record.status === 'failed' ? 'var(--color-error)' : 'var(--color-info)',
                            color: '#fff',
                            fontWeight: 600,
                          }}>
                            {record.status === 'success' ? '成功' : record.status === 'failed' ? '失败' : '进行中'}
                          </span>
                        </div>
                      </div>
                      <div style={{ display: 'flex', gap: 6, alignItems: 'center', flexWrap: 'wrap' }}>
                        {recExecutor && (
                          <Tag color={recExecutor.color} style={{ fontWeight: 600, fontSize: 10, padding: '0 6px', lineHeight: '18px' }}>
                            {recExecutor.icon} {recExecutor.label}
                          </Tag>
                        )}
                        {record.model && <Tag color="#3b82f6" style={{ fontSize: 10, padding: '0 6px', lineHeight: '18px' }}>{record.model}</Tag>}
                        <Tag color={record.trigger_type === 'cron' ? '#8b5cf6' : '#6b7280'} style={{ fontSize: 10, padding: '0 6px', lineHeight: '18px' }}>
                          {record.trigger_type === 'cron' ? 'Cron' : '手动'}
                        </Tag>
                        {record.usage?.duration_ms && (
                          <span style={{ fontSize: 10, color: 'var(--color-success)', fontWeight: 600 }}>
                            {(record.usage.duration_ms / 1000).toFixed(2)}s
                          </span>
                        )}
                        {record.execution_stats && (
                          <span style={{ fontSize: 10, color: 'var(--color-text-tertiary)' }}>
                            🔧{record.execution_stats.tool_calls} 💬{record.execution_stats.conversation_turns}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
              {historyTotal > historyLimit && (
                <div style={{ flexShrink: 0, display: 'flex', justifyContent: 'center', padding: '8px 0 0', borderTop: '1px solid var(--color-border-light)' }}>
                  <Pagination
                    simple
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
                  />
                </div>
              )}
            </div>
            {/* Divider */}
            <div style={{ width: 1, background: 'var(--color-border-light)', flexShrink: 0 }} />
            {/* Right: Record Detail */}
            <div className="history-detail-column">
              {selectedHistoryRecord ? (() => {
                const record = selectedHistoryRecord;
                const isRunning = record.status === 'running';
                const runningTask = isRunning ? getRunningTaskForRecord(record) : null;
                const liveLogs = runningTask ? runningTask.logs : null;
                const restLogs = parseRecordLogs(record);
                const displayLogs = liveLogs && liveLogs.length > 0 ? liveLogs : restLogs;
                return (
                  <>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12, flexWrap: 'wrap', gap: 8 }}>
                      <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
                        {record.executor && (() => {
                          const recOpt = getExecutorOption(record.executor);
                          return <Tag color={recOpt.color} style={{ fontWeight: 600 }}>{recOpt.icon} {recOpt.label}</Tag>;
                        })()}
                        {record.model && <Tag color="#3b82f6">{record.model}</Tag>}
                        <span style={{ fontSize: 13, color: 'var(--color-text-secondary)', fontWeight: 500 }}>
                          {formatLocalDateTime(record.started_at)}
                        </span>
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
                        {record.usage?.duration_ms && (
                          <span style={{ fontSize: 12, color: 'var(--color-success)', fontWeight: 600 }}>
                            {(record.usage.duration_ms / 1000).toFixed(2)}s
                          </span>
                        )}
                      </div>
                      <div style={{ display: 'flex', gap: 8 }}>
                        {record.status !== 'running' && supportsResume(record) && (
                          <Button type="primary" size="small" icon={<MessageOutlined />} onClick={() => handleOpenResume(record)}>继续对话</Button>
                        )}
                        {hasLogs(record) && (
                          <Button size="small" icon={<FileTextOutlined />} onClick={() => handleExportMarkdown(record)}>导出YAML</Button>
                        )}
                        {record.status === 'running' && (
                          <Popconfirm
                            title="确定强制停止该任务？"
                            okText="停止"
                            cancelText="取消"
                            onConfirm={async () => { await handleStopExecution(record.id); }}
                          >
                            <Button type="primary" danger size="small" icon={<StopOutlined />}>停止</Button>
                          </Popconfirm>
                        )}
                      </div>
                    </div>
                    {record.command && (
                      <div style={{ fontSize: 11, color: 'var(--color-text-quaternary)', marginBottom: 12, fontFamily: 'var(--font-mono)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {record.command}
                      </div>
                    )}
                    {record.result !== null && record.result !== '' && (
                      <div className={`history-result ${record.status === 'success' ? 'history-result-success' : 'history-result-failed'}`} style={{ marginBottom: 12 }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
                          <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-text)' }}>结论</span>
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
                      <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginBottom: 12, display: 'flex', gap: 12, flexWrap: 'wrap' }}>
                        <span>Input: {record.usage.input_tokens.toLocaleString()}</span>
                        <span>Output: {record.usage.output_tokens.toLocaleString()}</span>
                        {record.usage.total_cost_usd !== null && (
                          <span style={{ color: 'var(--color-warning)', fontWeight: 600 }}>${record.usage.total_cost_usd.toFixed(6)}</span>
                        )}
                      </div>
                    )}
                    {(() => {
                      const stats = resolveExecutionStats(record, isRunning);
                      if (!stats) return null;
                      return (
                        <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginBottom: 12, display: 'flex', gap: 12, flexWrap: 'wrap' }}>
                          <span>工具调用: <b style={{ color: 'var(--color-primary)' }}>{stats.tool_calls}</b></span>
                          <span>对话轮次: <b style={{ color: 'var(--color-primary)' }}>{stats.conversation_turns}</b></span>
                          {stats.thinking_count > 0 && (
                            <span>思考次数: <b style={{ color: 'var(--color-primary)' }}>{stats.thinking_count}</b></span>
                          )}
                        </div>
                      );
                    })()}
                    {(() => {
                      if (!isRunning && displayLogs.length === 0) return null;
                      if (viewMode === 'chat') {
                        return (
                          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8, flexShrink: 0 }}>
                              <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-primary)' }}>
                                对话视图 ({displayLogs.length} 条){isRunning && liveLogs && liveLogs.length > 0 ? ' · 实时' : ''}
                              </span>
                              <ReloadOutlined
                                style={{ fontSize: 12, color: 'var(--color-text-tertiary)', cursor: 'pointer' }}
                                onClick={() => refreshSingleRecord(record.id)}
                              />
                            </div>
                            <ChatView logs={displayLogs as LogEntry[]} isRunning={isRunning} />
                          </div>
                        );
                      }
                      return (
                        <div>
                          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                            <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-primary)' }}>
                              执行过程 ({displayLogs.length} 条){isRunning && liveLogs && liveLogs.length > 0 ? ' · 实时' : ''}
                            </span>
                            <ReloadOutlined
                              style={{ fontSize: 12, color: 'var(--color-text-tertiary)', cursor: 'pointer' }}
                              onClick={() => refreshSingleRecord(record.id)}
                            />
                          </div>
                          <div style={{
                            background: 'var(--log-bg)',
                            color: 'var(--log-text)',
                            padding: 12,
                            borderRadius: 8,
                            fontFamily: 'var(--font-mono)',
                            fontSize: 11,
                            overflow: 'auto',
                          }}>
                            {displayLogs.length === 0 ? (
                              <div style={{ color: 'var(--log-text-muted)' }}>等待输出...</div>
                            ) : (
                              displayLogs.map((log, idx) => (
                                <div key={idx} style={{ marginBottom: 4, display: 'flex', gap: 8 }}>
                                  <span style={{ color: 'var(--log-text-muted)', flexShrink: 0 }}>{formatLogTime(log.timestamp || '')}</span>
                                  <span style={{ color: logTypeColors[log.type || ''] || 'var(--log-text)' }}>
                                    [{logTypeLabels[log.type || ''] || log.type}]
                                  </span>
                                  <span>{log.content}</span>
                                </div>
                              ))
                            )}
                          </div>
                        </div>
                      );
                    })()}
                  </>
                );
              })() : (
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
                  <Empty description="选择一条执行记录查看详情" image={Empty.PRESENTED_IMAGE_SIMPLE} />
                </div>
              )}
            </div>
          </div>
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
                    {record.status !== 'running' && supportsResume(record) && (
                      <Button
                        type="primary"
                        size="small"
                        icon={<MessageOutlined />}
                        onClick={() => handleOpenResume(record)}
                      >
                        继续对话
                      </Button>
                    )}
                    {hasLogs(record) && (
                      <Button size="small" icon={<FileTextOutlined />} onClick={() => handleExportMarkdown(record)}>导出YAML</Button>
                    )}
                    {record.status === 'running' && (() => {
                      return (
                        <Popconfirm
                          title="确定强制停止该任务？"
                          okText="停止"
                          cancelText="取消"
                          onConfirm={async () => {
                            await handleStopExecution(record.id);
                          }}
                        >
                          <Button
                            type="primary"
                            danger
                            size="middle"
                            icon={<StopOutlined />}
                            style={{
                              fontSize: 14,
                              fontWeight: 600,
                              height: '32px',
                              display: 'flex',
                              alignItems: 'center',
                              gap: '6px',
                            }}
                          >
                            停止任务
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
                  const stats = resolveExecutionStats(record, isRunning);
                  if (!stats) return null;
                  return (
                    <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginTop: 8, display: 'flex', gap: 12, flexWrap: 'wrap' }}>
                      <span>工具调用: <b style={{ color: 'var(--color-primary)' }}>{stats.tool_calls}</b></span>
                      <span>对话轮次: <b style={{ color: 'var(--color-primary)' }}>{stats.conversation_turns}</b></span>
                      {stats.thinking_count > 0 && (
                        <span>思考次数: <b style={{ color: 'var(--color-primary)' }}>{stats.thinking_count}</b></span>
                      )}
                    </div>
                  );
                })()}

                {(() => {
                  const isRunning = record.status === 'running';
                  const runningTask = isRunning ? getRunningTaskForRecord(record) : null;
                const liveLogs = runningTask ? runningTask.logs : null;
                  const restLogs = parseRecordLogs(record);
                  const displayLogs = liveLogs && liveLogs.length > 0 ? liveLogs : restLogs;

                  if (!isRunning && displayLogs.length === 0) return null;

                  if (viewMode === 'chat') {
                    return (
                      <div style={{ marginTop: 8 }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                          <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--color-primary)' }}>
                            对话视图 ({displayLogs.length} 条){isRunning && liveLogs && liveLogs.length > 0 ? ' · 实时' : ''}
                          </span>
                          <ReloadOutlined
                            style={{ fontSize: 11 }}
                            onClick={(e) => { e.preventDefault(); e.stopPropagation(); refreshSingleRecord(record.id); }}
                          />
                        </div>
                        <div style={{ maxHeight: 400, overflow: 'auto' }}>
                          <ChatView logs={displayLogs as LogEntry[]} isRunning={isRunning} />
                        </div>
                      </div>
                    );
                  }

                  return (
                    <details style={{ marginTop: 8 }} open={isRunning}>
                      <summary style={{ cursor: 'pointer', color: 'var(--color-primary)', fontSize: 12, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 }}>
                        <span>查看日志 ({displayLogs.length} 条){isRunning && liveLogs && liveLogs.length > 0 ? ' · 实时' : ''}</span>
                        <ReloadOutlined
                          style={{ fontSize: 11 }}
                          onClick={(e) => { e.preventDefault(); e.stopPropagation(); refreshSingleRecord(record.id); }}
                        />
                      </summary>
                      <div style={{
                        background: 'var(--log-bg)',
                        color: 'var(--log-text)',
                        padding: 8,
                        borderRadius: 8,
                        fontFamily: 'var(--font-mono)',
                        fontSize: 11,
                        marginTop: 8,
                        maxHeight: 250,
                        overflow: 'auto',
                      }}>
                        {displayLogs.length === 0 ? (
                          <div style={{ color: 'var(--log-text-muted)' }}>等待输出...</div>
                        ) : (
                          displayLogs.map((log, idx) => (
                            <div key={idx} style={{ marginBottom: 4, display: 'flex', gap: 8 }}>
                              <span style={{ color: 'var(--log-text-muted)', flexShrink: 0 }}>{formatLogTime(log.timestamp || '')}</span>
                              <span style={{ color: logTypeColors[log.type || ''] || 'var(--log-text)' }}>
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

      <TodoEditDrawer
        open={isEditing}
        todo={selectedTodo}
        onClose={() => setIsEditing(false)}
        onSave={handleSaveEdit}
      />

      <TodoSettingsDrawer
        open={settingsOpen}
        todo={selectedTodo}
        tags={state.tags}
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

      <Modal
        title="继续对话"
        open={resumeModalOpen}
        onOk={handleResumeConfirm}
        onCancel={() => setResumeModalOpen(false)}
        confirmLoading={resumeLoading}
        okText="开始执行"
        cancelText="取消"
      >
        <p style={{ marginBottom: 12, color: 'var(--color-text-secondary)' }}>
          将复用之前的会话上下文继续对话，你可以修改下方消息：
        </p>
        <Input.TextArea
          value={resumeMessage}
          onChange={(e) => setResumeMessage(e.target.value)}
          rows={4}
          placeholder="输入要继续发送的消息..."
        />
      </Modal>
    </div>
  );
}

const logTypeColors: Record<string, string> = {
  info: '#60a5fa',
  text: '#4ade80',
  tool: '#fbbf24',
  tool_use: '#fbbf24',
  tool_call: '#fbbf24',
  tool_result: '#fbbf24',
  step_start: '#c084fc',
  step_finish: '#2dd4bf',
  stdout: '#cbd5e1',
  stderr: '#94a3b8',
  error: '#ef4444',
  system: '#94a3b8',
  assistant: '#a78bfa',
  user: '#22d3ee',
  result: '#4ade80',
  thinking: '#fb923c',
  tokens: '#94a3b8',
};

const logTypeLabels: Record<string, string> = {
  info: 'INFO',
  text: 'TEXT',
  tool: 'TOOL',
  tool_use: 'TOOL',
  tool_call: 'TOOL',
  tool_result: 'RESULT',
  step_start: 'START',
  step_finish: 'END',
  stdout: 'OUT',
  stderr: 'LOG',
  error: 'ERROR',
  system: 'SYS',
  assistant: 'ASST',
  user: 'USER',
  result: 'RESULT',
  thinking: 'THINK',
  tokens: 'INFO',
};

/**
 * 格式化时间戳为短时间格式 (HH:mm:ss)
 */
function formatLogTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  } catch {
    return iso;
  }
}
