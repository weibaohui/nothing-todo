import { useEffect, useState } from 'react';
import { Card, Tag, Segmented, Skeleton, Empty, Typography } from 'antd';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  ThunderboltOutlined,
  RobotOutlined,
  ExpandOutlined,
  CompressOutlined,
} from '@ant-design/icons';
import { useApp } from '../hooks/useApp';
import { ExecutorBadge } from './ExecutorBadge';
import * as db from '../utils/database';
import { formatRelativeTime } from '../utils/datetime';
import type { RecentCompletedTodo } from '../types';

const { Paragraph, Text } = Typography;

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
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(0)}s`;
  return `${(ms / 60_000).toFixed(1)}m`;
}

interface MemorialBoardProps {
  onBack?: () => void;
}

export function MemorialBoard({ onBack: _onBack }: MemorialBoardProps) {
  const { state, dispatch } = useApp();
  const [items, setItems] = useState<RecentCompletedTodo[]>([]);
  const [loading, setLoading] = useState(true);
  const [hours, setHours] = useState(24);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());

  useEffect(() => {
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
  }, [hours]);

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

  const handleSelectTodo = (todoId: number) => {
    dispatch({ type: 'SELECT_TODO', payload: todoId });
  };

  const successCount = items.filter(i => i.execution_status === 'success').length;
  const failedCount = items.filter(i => i.execution_status === 'failed').length;

  const renderResult = (result: string | null, expanded: boolean) => {
    if (!result) return <Text type="secondary" italic>暂无结论</Text>;
    const preview = expanded ? result : result.slice(0, 300);
    const needExpand = result.length > 300;
    return (
      <div className="memorial-result">
        <Paragraph
          style={{ margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}
          ellipsis={!expanded ? { rows: 4, expandable: false } : false}
        >
          {preview}
        </Paragraph>
        {needExpand && (
          <span className="memorial-expand-hint">
            {expanded ? (
              <><CompressOutlined /> 收起</>
            ) : (
              <><ExpandOutlined /> 展开完整结论</>
            )}
          </span>
        )}
      </div>
    );
  };

  if (loading) {
    return (
      <div className="memorial-board">
        <div className="memorial-header">
          <div className="memorial-header-top">
            <h2 className="memorial-title">看板</h2>
            <Segmented
              size="small"
              options={TIME_OPTIONS.map(o => o.label)}
              value={TIME_OPTIONS.find(o => o.value === hours)?.label || '24h'}
              onChange={label => {
                const opt = TIME_OPTIONS.find(o => o.label === label);
                if (opt) setHours(opt.value);
              }}
            />
          </div>
        </div>
        <div className="memorial-list">
          {[1, 2, 3].map(i => (
            <Card key={i} className="memorial-card" size="small">
              <Skeleton active paragraph={{ rows: 3 }} />
            </Card>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="memorial-board">
      <div className="memorial-header">
        <div className="memorial-header-top">
          <h2 className="memorial-title">看板</h2>
          <Segmented
            size="small"
            options={TIME_OPTIONS.map(o => o.label)}
            value={TIME_OPTIONS.find(o => o.value === hours)?.label || '24h'}
            onChange={label => {
              const opt = TIME_OPTIONS.find(o => o.label === label);
              if (opt) setHours(opt.value);
            }}
          />
        </div>
        <div className="memorial-summary">
          <span className="memorial-summary-item">
            共 <strong>{items.length}</strong> 条
          </span>
          <span className="memorial-summary-item memorial-summary-success">
            <CheckCircleOutlined style={{ marginRight: 2 }} />
            <strong>{successCount}</strong> 成功
          </span>
          <span className="memorial-summary-item memorial-summary-failed">
            <CloseCircleOutlined style={{ marginRight: 2 }} />
            <strong>{failedCount}</strong> 失败
          </span>
        </div>
      </div>

      {items.length === 0 ? (
        <div className="memorial-empty">
          <Empty
            description={
              <span style={{ color: 'var(--color-text-tertiary)' }}>
                最近 {hours} 小时内暂无完成的任务
              </span>
            }
          />
        </div>
      ) : (
        <div className="memorial-list">
          {items.map(item => {
            const isSuccess = item.execution_status === 'success';
            const expanded = expandedIds.has(item.todo_id);

            return (
              <Card
                key={item.todo_id}
                className={`memorial-card ${expanded ? 'expanded' : ''}`}
                size="small"
                onClick={() => toggleExpand(item.todo_id)}
                hoverable
                style={{
                  borderLeft: `3px solid ${isSuccess ? '#22c55e' : '#ef4444'}`,
                }}
              >
                <div className="memorial-card-body">
                  {/* Title row */}
                  <div className="memorial-card-title-row">
                    <a
                      className="memorial-card-title"
                      onClick={e => {
                        e.stopPropagation();
                        handleSelectTodo(item.todo_id);
                      }}
                    >
                      {item.title}
                    </a>
                    <div className="memorial-card-meta">
                      {isSuccess ? (
                        <Tag color="success" icon={<CheckCircleOutlined />}>成功</Tag>
                      ) : (
                        <Tag color="error" icon={<CloseCircleOutlined />}>失败</Tag>
                      )}
                      {item.executor && <ExecutorBadge executor={item.executor} />}
                    </div>
                  </div>

                  {/* Info row */}
                  <div className="memorial-card-info">
                    <span className="memorial-info-item">
                      <ClockCircleOutlined /> {formatRelativeTime(item.completed_at)}
                    </span>
                    {item.model && (
                      <span className="memorial-info-item">
                        <RobotOutlined /> {item.model}
                      </span>
                    )}
                    {item.trigger_type && item.trigger_type !== 'manual' && (
                      <span className="memorial-info-item">
                        <ThunderboltOutlined /> {item.trigger_type === 'scheduler' ? '定时' : item.trigger_type}
                      </span>
                    )}
                    {item.usage && (
                      <>
                        <span className="memorial-info-item memorial-info-tokens">
                          {formatTokens(item.usage.input_tokens)}+{formatTokens(item.usage.output_tokens)} tokens
                        </span>
                        {item.usage.total_cost_usd != null && item.usage.total_cost_usd > 0 && (
                          <span className="memorial-info-item memorial-info-cost">
                            ${item.usage.total_cost_usd.toFixed(4)}
                          </span>
                        )}
                        {item.usage.duration_ms != null && (
                          <span className="memorial-info-item">
                            {formatDuration(item.usage.duration_ms)}
                          </span>
                        )}
                      </>
                    )}
                  </div>

                  {/* Tags row */}
                  {item.tag_ids.length > 0 && (
                    <div className="memorial-card-tags">
                      {item.tag_ids.map(tid => {
                        const tag = state.tags.find(t => t.id === tid);
                        if (!tag) return null;
                        return (
                          <Tag key={tid} color={tag.color} style={{ margin: 0 }}>
                            {tag.name}
                          </Tag>
                        );
                      })}
                    </div>
                  )}

                  {/* Result preview */}
                  <div className="memorial-card-result">
                    {renderResult(item.result, expanded)}
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
