import { Tag, Badge } from 'antd';
import { StarFilled } from '@ant-design/icons';
import { ExecutorBadge } from '@/components/ExecutorBadge';
import { formatLocalDateTime, formatDurationSec } from '@/utils/datetime';
import { getElapsedSeconds } from './helpers';
import { useState, useEffect } from 'react';
import type { ExecutionRecord } from '@/types';

/**
 * 提取帖子标题：取 result 第一行有效内容（跳过空行和纯 markdown 标记行），
 * 截取前 40 字。无 result 时：
 * - 正在执行 → "执行中"
 * - 已完成/失败/取消等 → 显示 todo 标题（如果有），否则显示状态
 */
function extractPostTitle(
  result: string | null | undefined,
  status: string,
  todoTitle: string,
): string {
  if (result && result.trim() !== '') {
    for (const line of result.split('\n')) {
      const cleaned = line.replace(/^[#*>+\-\s]+/, '').trim();
      if (cleaned) {
        if (cleaned.length > 40) return cleaned.substring(0, 40) + '...';
        return cleaned;
      }
    }
  }
  // 没有结论
  if (status === 'running') return '执行中';
  return todoTitle || (status === 'success' ? '执行成功' : '执行失败');
}

/**
 * 论坛帖子卡片 —— 一行一条执行记录。
 */
export function ForumPostCard({
  record,
  isSelected,
  onSelect,
  replyCount,
  todoTitle,
}: {
  record: ExecutionRecord;
  isSelected: boolean;
  onSelect: () => void;
  /** 追问数量，>0 时显示 badge */
  replyCount?: number;
  /** todo 标题，无结论时用于兜底显示 */
  todoTitle: string;
}) {
  const isRunning = record.status === 'running';
  const [elapsedSec, setElapsedSec] = useState(isRunning ? getElapsedSeconds(record.started_at) : 0);

  useEffect(() => {
    if (!isRunning) return;
    const tick = () => setElapsedSec(getElapsedSeconds(record.started_at));
    tick();
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  }, [isRunning, record.started_at]);

  const title = extractPostTitle(record.result, record.status, todoTitle);
  const statusColor =
    record.status === 'success' ? 'var(--color-success)' :
    record.status === 'failed' ? 'var(--color-error)' :
    'var(--color-info)';

  const statusText =
    record.status === 'success' ? '成功' :
    record.status === 'failed' ? '失败' :
    '进行中';

  return (
    <div
      onClick={onSelect}
      style={{
        padding: '10px 14px',
        marginBottom: 2,
        borderRadius: 6,
        cursor: 'pointer',
        background: isSelected ? 'var(--color-primary-bg)' : 'var(--color-bg-elevated)',
        border: `1px solid ${isSelected ? 'var(--color-primary)' : 'var(--color-border-light)'}`,
        transition: 'background 0.15s, border-color 0.15s',
      }}
    >
      {/* 第一行：#id + 标题 + 追问 badge + 状态 tag（最右） */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
        <span style={{ flexShrink: 0, fontSize: 11, color: 'var(--color-text-tertiary)', fontFamily: 'monospace' }}>
          #{record.id}
        </span>
        <span style={{
          fontSize: 13,
          fontWeight: 600,
          color: isRunning ? 'var(--color-info)' : 'var(--color-text)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          flex: 1,
          minWidth: 0,
        }}>
          {title}
        </span>
        {replyCount != null && replyCount > 0 && (
          <Badge count={replyCount} size="small" style={{ backgroundColor: 'var(--color-primary)' }} />
        )}
        <span style={{
          flexShrink: 0,
          fontSize: 10,
          padding: '1px 8px',
          borderRadius: 10,
          backgroundColor: statusColor,
          color: '#fff',
          fontWeight: 600,
          lineHeight: '18px',
        }}>
          {statusText}
        </span>
      </div>

      {/* 第二行：元信息 */}
      <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap', fontSize: 11 }}>
        <span style={{ color: 'var(--color-text-tertiary)' }}>
          {formatLocalDateTime(record.started_at)}
        </span>
        {record.executor && <ExecutorBadge executor={record.executor} />}
        {record.model && <Tag color="#3b82f6" style={{ fontSize: 10, padding: '0 6px', lineHeight: '18px', margin: 0 }}>{record.model}</Tag>}
        {record.rating != null && (
          <span style={{ color: '#faad14', fontWeight: 600, display: 'flex', alignItems: 'center', gap: 2 }}>
            <StarFilled style={{ fontSize: 10 }} />
            {record.rating}
          </span>
        )}
        <Tag color={record.trigger_type === 'cron' ? '#8b5cf6' : record.trigger_type?.startsWith('hook:') ? '#a855f7' : '#6b7280'} style={{ fontSize: 10, padding: '0 6px', lineHeight: '18px', margin: 0 }}>
          {record.trigger_type === 'cron' ? 'Cron' : record.trigger_type?.startsWith('hook:') ? 'Hook' : '手动'}
        </Tag>
        {!isRunning && record.usage?.duration_ms && (
          <span style={{ color: 'var(--color-success)', fontWeight: 600 }}>
            {formatDurationSec(record.usage.duration_ms / 1000)}
          </span>
        )}
        {isRunning && elapsedSec > 0 && (
          <span style={{ color: 'var(--color-info)', fontWeight: 600 }}>
            {formatDurationSec(elapsedSec)}
          </span>
        )}
        {record.execution_stats && (
          <span style={{ color: 'var(--color-text-tertiary)' }}>
            🔧{record.execution_stats.tool_calls} 💬{record.execution_stats.conversation_turns}
          </span>
        )}
      </div>
    </div>
  );
}
