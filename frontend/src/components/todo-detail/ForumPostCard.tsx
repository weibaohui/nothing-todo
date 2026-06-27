import { Tag, Badge } from 'antd';
import { ExecutorBadge } from '@/components/ExecutorBadge';
import { formatLocalDateTime, formatDurationSec } from '@/utils/datetime';
import { getElapsedSeconds } from './helpers';
import { useState, useEffect } from 'react';
import type { ExecutionRecord } from '@/types';

/**
 * 提取帖子标题：取 result 第一行（去除 markdown 标记），
 * 截取前 40 字。无 result 时返回 "执行中"。
 */
function extractPostTitle(result: string | null | undefined): string {
  if (!result || result.trim() === '') return '执行中';
  const firstLine = result.split('\n')[0];
  const cleaned = firstLine.replace(/^[#*>+\-\s]+/, '').trim();
  if (!cleaned) return '执行中';
  if (cleaned.length > 40) return cleaned.substring(0, 40) + '...';
  return cleaned;
}

/**
 * 论坛帖子卡片 —— 一行一条执行记录。
 */
export function ForumPostCard({
  record,
  isSelected,
  onSelect,
  replyCount,
}: {
  record: ExecutionRecord;
  isSelected: boolean;
  onSelect: () => void;
  /** 追问数量，>0 时显示 badge */
  replyCount?: number;
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

  const title = extractPostTitle(record.result);
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
      {/* 第一行：状态 + 标题 + 追问 badge */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
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
        <span style={{
          fontSize: 13,
          fontWeight: 600,
          color: isRunning ? 'var(--color-info)' : 'var(--color-text)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}>
          {title}
        </span>
        {replyCount != null && replyCount > 0 && (
          <Badge count={replyCount} size="small" style={{ backgroundColor: 'var(--color-primary)' }} />
        )}
      </div>

      {/* 第二行：元信息 */}
      <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap', fontSize: 11 }}>
        <span style={{ color: 'var(--color-text-tertiary)' }}>
          {formatLocalDateTime(record.started_at)}
        </span>
        {record.executor && <ExecutorBadge executor={record.executor} />}
        {record.model && <Tag color="#3b82f6" style={{ fontSize: 10, padding: '0 6px', lineHeight: '18px', margin: 0 }}>{record.model}</Tag>}
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
