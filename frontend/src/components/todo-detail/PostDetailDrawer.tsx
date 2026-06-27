import { useState, useEffect } from 'react';
import { Drawer, Button, Tag, Empty, Segmented, Popconfirm, Tooltip, Pagination, message, Popover, InputNumber, Space } from 'antd';
import { StarOutlined, StarFilled, LinkOutlined, UnorderedListOutlined, CodeOutlined } from '@ant-design/icons';
import { MessageOutlined, FileTextOutlined, StopOutlined } from '@ant-design/icons';
import { ExecutorBadge } from '@/components/ExecutorBadge';
import { ChatView } from '@/components/ChatView';
import { CommandPanel } from '@/components/CommandPanel';
import { RefreshBtn } from './LogViewHeader';
import { formatLocalDateTime, formatDurationSec } from '@/utils/datetime';
import { getElapsedSeconds, formatLogTime } from './helpers';
import { LOG_TYPE_COLORS, LOG_TYPE_LABELS } from '@/constants';
import { CollapsibleConclusion } from './CollapsibleConclusion';
import { ReplyInput } from './ReplyInput';
import { copyToClipboard } from '@/utils/clipboard';
import { supportsResume } from '@/types';
import type { SessionGroup } from './helpers';
import type { ExecutionRecord, LogEntry, ExecutionStats } from '@/types';

export interface PostDetailProps {
  record: ExecutionRecord;
  sessionGroups: SessionGroup[];
  onSelectRecord: (id: number) => void;
  onStop: (recordId: number) => Promise<void>;
  onRefreshSingle: (recordId: number) => Promise<void>;
  onRate: (recordId: number, rating: number | null) => Promise<void>;
  onExportMarkdown: (record: ExecutionRecord) => Promise<void>;
  onReply: (record: ExecutionRecord, message: string) => Promise<void>;
  replyLoading: boolean;
  paginatedLogs: LogEntry[];
  logsTotal: number;
  logsPage: number;
  logsPerPage: number;
  onLoadLogs: (recordId: number, page: number) => Promise<void>;
  isLoadingLogs: boolean;
  getRunningTaskForRecord: (record: ExecutionRecord) => any;
  resolveExecutionStats: (record: ExecutionRecord, isRunning: boolean) => ExecutionStats | null | undefined;
}

/**
 * 帖子详情内容（无外壳）—— 供 PostDetailDrawer 和 TodoPostPage 共用。
 */
export function PostDetailContent(props: PostDetailProps) {
  const { record } = props;
  const [viewMode, setViewMode] = useState<'log' | 'chat' | 'command'>('log');
  const isRunning = record.status === 'running';
  const runningTask = isRunning ? props.getRunningTaskForRecord(record) : null;
  const liveLogs = runningTask ? runningTask.logs : null;
  const displayLogs = liveLogs && liveLogs.length > 0 ? liveLogs : props.paginatedLogs;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ flex: 1, overflow: 'auto', minHeight: 0 }}>
        {/* Continuation breadcrumb */}
        {(() => {
          const group = props.sessionGroups.find(g => g.records.some(r => r.id === record.id));
          if (!group || group.records.length <= 1 || !group.records[0].session_id) return null;
          const idx = group.records.findIndex(r => r.id === record.id);
          if (idx <= 0) return null;
          return (
            <div style={{
              display: 'flex', alignItems: 'center', gap: 6,
              marginBottom: 10, padding: '4px 10px', borderRadius: 6,
              background: 'var(--color-bg-elevated)', border: '1px solid var(--color-border-light)',
              fontSize: 11, color: 'var(--color-text-tertiary)',
            }}>
              <LinkOutlined style={{ color: 'var(--color-primary)', fontSize: 11 }} />
              <span>继续自</span>
              <span
                onClick={() => props.onSelectRecord(group.records[0].id)}
                style={{ cursor: 'pointer', color: 'var(--color-primary)', fontWeight: 500 }}
              >
                {formatLocalDateTime(group.records[0].started_at)}
              </span>
              {record.resume_message && (
                <>
                  <span style={{ color: 'var(--color-border)' }}>·</span>
                  <span style={{ color: 'var(--color-text-secondary)', fontStyle: 'italic' }}>
                    &quot;{String(record.resume_message).length > 40 ? String(record.resume_message).substring(0, 40) + '...' : record.resume_message}&quot;
                  </span>
                </>
              )}
              <span style={{ marginLeft: 'auto', color: 'var(--color-text-quaternary)' }}>
                第{idx + 1}轮 / 共{group.records.length}轮
              </span>
            </div>
          );
        })()}

        {/* Header row */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12, flexWrap: 'wrap', gap: 8 }}>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
            {record.executor && <ExecutorBadge executor={record.executor} />}
            {record.model && <Tag color="#3b82f6">{record.model}</Tag>}
            <span style={{ fontSize: 13, color: 'var(--color-text-secondary)', fontWeight: 500 }}>
              {formatLocalDateTime(record.started_at)}
            </span>
            <span style={{
              fontSize: 11, padding: '3px 12px', borderRadius: 12,
              backgroundColor: record.status === 'success' ? 'var(--color-success)' : record.status === 'failed' ? 'var(--color-error)' : 'var(--color-info)',
              color: '#fff', fontWeight: 600,
            }}>
              {record.status === 'success' ? '成功' : record.status === 'failed' ? '失败' : '进行中'}
            </span>
            {!isRunning && record.usage?.duration_ms && (
              <span style={{ fontSize: 12, color: 'var(--color-success)', fontWeight: 600 }}>
                {formatDurationSec(record.usage.duration_ms / 1000)}
              </span>
            )}
            {isRunning && (
              <span style={{ fontSize: 12, color: 'var(--color-info)', fontWeight: 600 }}>
                {formatDurationSec(getElapsedSeconds(record.started_at))}
              </span>
            )}
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            {!isRunning && supportsResume(record) && (
              <Button type="primary" size="small" icon={<MessageOutlined />} onClick={() => props.onReply(record, '')}>继续对话</Button>
            )}
            {!isRunning && (
              <RatingControl record={record} onRate={props.onRate} />
            )}
            {!isRunning && !!record.finished_at && (
              <Button type="text" size="small" icon={<FileTextOutlined />} onClick={() => props.onExportMarkdown(record)}>导出YAML</Button>
            )}
            {isRunning && (
              <Popconfirm title="确定强制停止该任务？" okText="停止" cancelText="取消" onConfirm={async () => { await props.onStop(record.id); }}>
                <Button type="text" size="small" icon={<StopOutlined />}>停止</Button>
              </Popconfirm>
            )}
          </div>
        </div>

        {/* Command */}
        {record.command && (
          <Tooltip title="点击复制命令">
            <div
              onClick={async () => {
                try {
                  const ok = await copyToClipboard(record.command || '');
                  message[ok ? 'success' : 'error'](ok ? '已复制' : '复制失败');
                } catch { message.error('复制失败'); }
              }}
              style={{ fontSize: 11, color: 'var(--color-text-quaternary)', marginBottom: 12, fontFamily: 'var(--font-mono)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', cursor: 'pointer' }}
            >
              {record.command}
            </div>
          </Tooltip>
        )}

        {/* Worktree path */}
        {record.worktree_path && (
          <div style={{ fontSize: 11, color: 'var(--color-text-quaternary)', marginBottom: 12 }}>
            worktree: {record.worktree_path}
          </div>
        )}

        {/* Conclusion */}
        {record.result !== null && record.result !== '' && (
          <CollapsibleConclusion
            result={record.result}
            status={record.status}
            messageApi={message}
            showTitle
            recordId={record.id}
          />
        )}

        {/* Usage stats */}
        {record.usage && (
          <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginBottom: 12, display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <span>Input: {record.usage.input_tokens.toLocaleString()}</span>
            <span>Output: {record.usage.output_tokens.toLocaleString()}</span>
            {record.usage.total_cost_usd !== null && (
              <span style={{ color: 'var(--color-warning)', fontWeight: 600 }}>${record.usage.total_cost_usd.toFixed(6)}</span>
            )}
          </div>
        )}

        {/* Execution stats */}
        {(() => {
          const stats = props.resolveExecutionStats(record, isRunning);
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

        {/* Log view header + content */}
        {(() => {
          if (!isRunning && displayLogs.length === 0) return null;
          return (
            <div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-primary)' }}>
                    {viewMode === 'command'
                      ? `命令视图 (${displayLogs.length} 条${isRunning && liveLogs && liveLogs.length > 0 ? ' · 实时' : ''})`
                      : viewMode === 'chat'
                        ? `对话视图 (${displayLogs.length} 条${isRunning && liveLogs && liveLogs.length > 0 ? ' · 实时' : ''})`
                        : `执行过程 (${isRunning ? displayLogs.length : props.logsTotal} 条${isRunning && liveLogs && liveLogs.length > 0 ? ' · 实时' : ''})`}
                  </span>
                  <RefreshBtn onClick={() => { props.onRefreshSingle(record.id); props.onLoadLogs(record.id, props.logsPage); }} />
                </div>
                <Segmented
                  size="small"
                  value={viewMode}
                  onChange={(value) => setViewMode(value as 'log' | 'chat' | 'command')}
                  options={[
                    { value: 'log', icon: <UnorderedListOutlined />, label: '日志' },
                    { value: 'chat', icon: <MessageOutlined />, label: '对话' },
                    { value: 'command', icon: <CodeOutlined />, label: '命令' },
                  ]}
                />
              </div>

              {viewMode === 'chat' ? (
                <ChatView logs={displayLogs as LogEntry[]} isRunning={isRunning} />
              ) : viewMode === 'command' ? (
                <div style={{ overflow: 'auto', padding: 4 }}>
                  <CommandPanel logs={displayLogs} executor={record.executor} />
                </div>
              ) : (
                <div style={{
                  background: 'var(--log-bg)', color: 'var(--log-text)',
                  padding: 12, borderRadius: 8, fontFamily: 'var(--font-mono)', fontSize: 11, overflow: 'auto',
                }}>
                  {displayLogs.length === 0 ? (
                    <div style={{ color: 'var(--log-text-muted)' }}>{isRunning ? '等待输出...' : (props.isLoadingLogs ? '加载中...' : '暂无日志')}</div>
                  ) : (
                    displayLogs.map((log: LogEntry, idx: number) => (
                      <div key={idx} style={{ marginBottom: 4, display: 'flex', gap: 8 }}>
                        <span style={{ color: 'var(--log-text-muted)', flexShrink: 0 }}>{formatLogTime(log.timestamp || '')}</span>
                        <span style={{ color: LOG_TYPE_COLORS[log.type || ''] || 'var(--log-text)' }}>
                          [{LOG_TYPE_LABELS[log.type || ''] || log.type}]
                        </span>
                        <span>{log.content}</span>
                      </div>
                    ))
                  )}
                </div>
              )}

              {!isRunning && props.logsTotal > props.logsPerPage && (
                <Pagination
                  simple
                  current={props.logsPage}
                  pageSize={props.logsPerPage}
                  total={props.logsTotal}
                  onChange={(page) => props.onLoadLogs(record.id, page)}
                  size="small"
                  style={{ marginTop: 8, textAlign: 'center' }}
                />
              )}
            </div>
          );
        })()}
      </div>

      {/* Reply input at the bottom */}
      {!isRunning && supportsResume(record) && (
        <div style={{ flexShrink: 0, paddingTop: 12, borderTop: '1px solid var(--color-border-light)' }}>
          <ReplyInput record={record} onReply={props.onReply} loading={props.replyLoading} />
        </div>
      )}
    </div>
  );
}

/**
 * 帖子详情抽屉 —— 点击帖子后从右侧滑出。
 */
export function PostDetailDrawer({
  open,
  record,
  onClose,
  ...contentProps
}: {
  open: boolean;
  record: ExecutionRecord | null;
  onClose: () => void;
} & PostDetailProps) {
  if (!record) {
    return (
      <Drawer
        title="帖子详情"
        open={open}
        onClose={onClose}
        width="65%"
        styles={{ body: { padding: 0 } }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
          <Empty description="请选择一条执行记录" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        </div>
      </Drawer>
    );
  }

  return (
    <Drawer
      title={
        <span style={{ fontSize: 14, fontWeight: 600 }}>
          {record.result
            ? (record.result.split('\n')[0]?.replace(/^[#*>+\-\s]+/, '').trim().substring(0, 50) || '执行记录')
            : '执行记录'}
          {record.result && record.result.split('\n')[0]?.length > 50 ? '...' : ''}
        </span>
      }
      open={open}
      onClose={onClose}
      width="65%"
      styles={{ body: { padding: '16px 20px', display: 'flex', flexDirection: 'column' } }}
    >
      <PostDetailContent record={record} {...contentProps} />
    </Drawer>
  );
}

/**
 * 评分控件，与 RecordDetailView 内部的 RecordRatingControl 功能一致。
 */
function RatingControl({
  record,
  onRate,
}: {
  record: ExecutionRecord;
  onRate: (recordId: number, rating: number | null) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [value, setValue] = useState<number | null>(record.rating ?? null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    setValue(record.rating ?? null);
  }, [record.rating, record.id]);

  const handleSubmit = async (next: number | null) => {
    setSubmitting(true);
    try {
      await onRate(record.id, next);
      setOpen(false);
    } finally {
      setSubmitting(false);
    }
  };

  if (record.rating != null) {
    return (
      <Popover
        open={open}
        onOpenChange={setOpen}
        trigger="click"
        content={
          <Space.Compact style={{ width: 200 }}>
            <InputNumber
              min={0} max={100} value={value}
              onChange={v => setValue(typeof v === 'number' ? v : null)}
              placeholder="0-100" style={{ width: '100%' }}
              onPressEnter={() => { if (value != null) handleSubmit(value); }}
            />
            <Button type="primary" loading={submitting} onClick={() => { if (value != null) handleSubmit(value); }}>
              更新
            </Button>
          </Space.Compact>
        }
      >
        <Button type="text" size="small" icon={<StarFilled style={{ color: '#faad14' }} />}>
          {record.rating}
        </Button>
      </Popover>
    );
  }

  return (
    <Popover
      open={open}
      onOpenChange={setOpen}
      trigger="click"
      content={
        <Space.Compact style={{ width: 200 }}>
          <InputNumber
            min={0} max={100} value={value}
            onChange={v => setValue(typeof v === 'number' ? v : null)}
            placeholder="0-100" style={{ width: '100%' }}
            onPressEnter={() => { if (value != null) handleSubmit(value); }}
          />
          <Button type="primary" loading={submitting} onClick={() => { if (value != null) handleSubmit(value); }}>
            评分
          </Button>
        </Space.Compact>
      }
    >
      <Button type="text" size="small" icon={<StarOutlined />}>评分</Button>
    </Popover>
  );
}
