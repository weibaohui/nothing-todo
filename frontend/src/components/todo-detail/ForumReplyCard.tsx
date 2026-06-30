import { LinkOutlined } from '@ant-design/icons';
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
  if (status === 'running') return '执行中';
  return todoTitle || (status === 'success' ? '执行成功' : '执行失败');
}

/**
 * 论坛跟帖/回复卡片 —— 同 session 的后续执行记录。
 * 缩进 + 左边框连线，视觉上表示主帖的回复。
 */
export function ForumReplyCard({
  record,
  resumeMessage,
  isSelected,
  onSelect,
  todoTitle,
}: {
  record: ExecutionRecord;
  resumeMessage?: string | null;
  isSelected: boolean;
  onSelect: () => void;
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
  const msg = resumeMessage
    ? (resumeMessage.length > 30 ? resumeMessage.substring(0, 30) + '...' : resumeMessage)
    : '继续对话';

  const statusColor =
    record.status === 'success' ? 'var(--color-success)' :
    record.status === 'failed' ? 'var(--color-error)' :
    'var(--color-info)';

  return (
    <div
      onClick={onSelect}
      style={{
        marginLeft: 24,
        padding: '6px 10px',
        borderLeft: '2px solid var(--color-primary)',
        borderBottom: '1px solid var(--color-border-light)',
        cursor: 'pointer',
        background: isSelected ? 'var(--color-primary-bg)' : 'var(--color-bg-elevated)',
        transition: 'background 0.15s',
        marginBottom: 1,
      }}
    >
      {/* 第一行：图标 + resume_message + 状态 */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 3 }}>
        <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 11, color: 'var(--color-primary)', fontWeight: 500 }}>
          <LinkOutlined style={{ fontSize: 10 }} />
          <span style={{ color: 'var(--color-text-secondary)', fontWeight: 400 }}>
            {msg}
          </span>
        </span>
        <span style={{
          flexShrink: 0,
          fontSize: 9,
          padding: '1px 6px',
          borderRadius: 8,
          backgroundColor: statusColor,
          color: '#fff',
          fontWeight: 600,
          lineHeight: '16px',
        }}>
          {record.status === 'success' ? '成功' : record.status === 'failed' ? '失败' : '进行中'}
        </span>
      </div>

      {/* 第二行：标题 */}
      <div style={{
        fontSize: 12,
        fontWeight: 500,
        color: isRunning ? 'var(--color-info)' : 'var(--color-text)',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap',
        marginBottom: 3,
        paddingLeft: 14,
      }}>
        {title}
      </div>

      {/* 第三行：元信息 */}
      <div style={{ display: 'flex', gap: 4, alignItems: 'center', flexWrap: 'wrap', fontSize: 10, paddingLeft: 14 }}>
        <span style={{ color: 'var(--color-text-tertiary)' }}>
          {formatLocalDateTime(record.started_at)}
        </span>
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
            🔧{record.execution_stats.tool_calls}
          </span>
        )}
      </div>
    </div>
  );
}
