import { useEffect, useState } from 'react';
import { Card, Tag, Segmented, Skeleton, Empty, Badge, message, Button } from 'antd';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  RobotOutlined,
  CopyOutlined,
  LeftOutlined,
  AppstoreOutlined,
  ProfileOutlined,
} from '@ant-design/icons';
import XMarkdown from '@ant-design/x-markdown';
import { useApp } from '../hooks/useApp';
import { ExecutorBadge } from './ExecutorBadge';
import { KanbanBoard } from './KanbanBoard';
import * as db from '../utils/database';
import { formatRelativeTime } from '../utils/datetime';
import type { RecentCompletedTodo } from '../types';

const TIME_OPTIONS: { label: string; value: number }[] = [
  { label: '6h', value: 6 },
  { label: '12h', value: 12 },
  { label: '24h', value: 24 },
  { label: '3d', value: 72 },
  { label: '7d', value: 168 },
];

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function formatDuration(ms: number): string {
  if (ms < 1_000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1_000).toFixed(0)}s`;
  return `${(ms / 60_000).toFixed(1)}m`;
}

interface MemorialBoardProps {
  onBack?: () => void;
}

type BoardMode = 'memorial' | 'kanban';

export function MemorialBoard({ onBack }: MemorialBoardProps) {
  const { state, dispatch } = useApp();
  const { onSelectTodo } = { onSelectTodo: (todoId: number) => {
    dispatch({ type: 'SELECT_TODO', payload: todoId });
  } };
  const [boardMode, setBoardMode] = useState<BoardMode>('memorial');
  const [items, setItems] = useState<RecentCompletedTodo[]>([]);
  const [loading, setLoading] = useState(true);
  const [hours, setHours] = useState(24);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());

  useEffect(() => {
    if (boardMode !== 'memorial') return;
    let cancelled = false;
    setLoading(true);
    db.getRecentCompletedTodos(hours)
      .then(data => {
        if (!cancelled) setItems(data);
      })
      .catch(() => {
        if (!cancelled) setItems([]);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => { cancelled = true; };
  }, [hours, boardMode]);

  const toggleExpand = (todoId: number) => {
    setExpandedIds(prev => {
      const next = new Set(prev);
      if (next.has(todoId)) {
        next.delete(todoId);
      } else {
        next.add(todoId);
      }
      return next;
    });
  };

  const handleSelectTodo = (todoId: number, e: React.MouseEvent) => {
    e.stopPropagation();
    dispatch({ type: 'SELECT_TODO', payload: todoId });
  };

  const successCount = items.filter(i => i.execution_status === 'success').length;
  const failedCount = items.filter(i => i.execution_status === 'failed').length;

  const renderCard = (item: RecentCompletedTodo) => {
    const isSuccess = item.execution_status === 'success';
    const expanded = expandedIds.has(item.todo_id);
    const result = item.result || '';
    const previewMd = result.length > 200 && !expanded ? result.slice(0, 200) + '…' : result;

    return (
      <Card
        key={item.todo_id}
        className={`memorial-card ${expanded ? 'expanded' : ''}`}
        size="small"
        onClick={() => toggleExpand(item.todo_id)}
        style={{
          borderTop: `3px solid ${isSuccess ? '#22c55e' : '#ef4444'}`,
        }}
        bodyStyle={{ padding: 0 }}
      >
        {/* Card Header */}
        <div className="memorial-card-header">
          <div className="memorial-card-top">
            <span
              className="memorial-card-title"
              onClick={e => handleSelectTodo(item.todo_id, e)}
              title={item.title}
            >
              {item.title}
            </span>
            {isSuccess ? (
              <CheckCircleOutlined className="memorial-status-icon memorial-success" />
            ) : (
              <CloseCircleOutlined className="memorial-status-icon memorial-failed" />
            )}
          </div>
          <div className="memorial-card-meta-row">
            {item.executor && <ExecutorBadge executor={item.executor} />}
            <span className="memorial-meta-time">
              <ClockCircleOutlined /> {formatRelativeTime(item.completed_at)}
            </span>
            {item.model && (
              <span className="memorial-meta-model">
                <RobotOutlined /> {item.model}
              </span>
            )}
          </div>
          {item.tag_ids.length > 0 && (
            <div className="memorial-card-tags">
              {item.tag_ids.map(tid => {
                const tag = state.tags.find(t => t.id === tid);
                if (!tag) return null;
                return (
                  <Tag key={tid} color={tag.color} className="memorial-tag">
                    {tag.name}
                  </Tag>
                );
              })}
            </div>
          )}
        </div>

        {/* Card Footer — Result */}
        <div className="memorial-card-footer">
          {result ? (
            <div className={`memorial-result ${expanded ? 'expanded' : ''}`}>
              <button
                className="memorial-copy-btn"
                onClick={e => {
                  e.stopPropagation();
                  navigator.clipboard.writeText(result).then(() => message.success('已复制'));
                }}
                title="复制结论"
              >
                <CopyOutlined />
              </button>
              <XMarkdown content={previewMd} />
              {result.length > 200 && (
                <button className="memorial-expand-btn" onClick={e => { e.stopPropagation(); toggleExpand(item.todo_id); }}>
                  {expanded ? '收起' : '展开'}
                </button>
              )}
            </div>
          ) : (
            <span className="memorial-no-result">暂无结论</span>
          )}

          {/* Usage stats */}
          {item.usage && (
            <div className="memorial-usage-row">
              {item.usage.duration_ms != null && (
                <span className="memorial-stat">{formatDuration(item.usage.duration_ms)}</span>
              )}
              <span className="memorial-stat memorial-tokens">
                {formatTokens(item.usage.input_tokens)} + {formatTokens(item.usage.output_tokens)} tokens
              </span>
              {item.usage.total_cost_usd != null && item.usage.total_cost_usd > 0 && (
                <span className="memorial-stat memorial-cost">
                  ${item.usage.total_cost_usd.toFixed(4)}
                </span>
              )}
              {item.trigger_type && item.trigger_type !== 'manual' && (
                <Badge
                  count={item.trigger_type === 'scheduler' ? '定时' : item.trigger_type}
                  style={{ fontSize: 10, height: 16, lineHeight: '16px' }}
                />
              )}
            </div>
          )}
        </div>
      </Card>
    );
  };

  return (
    <div className="memorial-board">
      <div className="memorial-header">
        <div className="memorial-header-top">
          {onBack && (
            <Button
              type="text"
              size="small"
              icon={<LeftOutlined />}
              onClick={onBack}
              className="memorial-back-btn"
              aria-label="返回"
            />
          )}
          <h2 className="memorial-title">看板</h2>
          <Segmented
            size="small"
            value={boardMode}
            onChange={value => setBoardMode(value as BoardMode)}
            options={[
              { label: <span><ProfileOutlined /> 执行结论</span>, value: 'memorial' },
              { label: <span><AppstoreOutlined /> 飞书看板</span>, value: 'kanban' },
            ]}
          />
          {boardMode === 'memorial' && (
            <Segmented
              size="small"
              options={TIME_OPTIONS.map(o => ({ label: o.label, value: o.label }))}
              value={TIME_OPTIONS.find(o => o.value === hours)?.label || '24h'}
              onChange={label => {
                const opt = TIME_OPTIONS.find(o => o.label === label);
                if (opt) setHours(opt.value);
              }}
            />
          )}
        </div>
        {boardMode === 'memorial' && (
          <div className="memorial-summary">
            <span className="memorial-stat-dot memorial-stat-all">共 <strong>{items.length}</strong> 条</span>
            <span className="memorial-stat-dot memorial-stat-success">
              <CheckCircleOutlined /> <strong>{successCount}</strong> 成功
            </span>
            <span className="memorial-stat-dot memorial-stat-failed">
              <CloseCircleOutlined /> <strong>{failedCount}</strong> 失败
            </span>
          </div>
        )}
      </div>

      {boardMode === 'kanban' ? (
        <KanbanBoard onSelectTodo={onSelectTodo} />
      ) : loading ? (
        <div className="memorial-grid">
          {[1, 2, 3, 4].map(i => (
            <Card key={i} className="memorial-card" size="small" bodyStyle={{ padding: 12 }}>
              <Skeleton active paragraph={{ rows: 4 }} />
            </Card>
          ))}
        </div>
      ) : items.length === 0 ? (
        <div className="memorial-empty">
          <Empty description={<span style={{ color: 'var(--color-text-tertiary)' }}>最近 {hours} 小时内暂无完成的任务</span>} />
        </div>
      ) : (
        <div className="memorial-grid">
          {items.map(renderCard)}
        </div>
      )}
    </div>
  );
}
