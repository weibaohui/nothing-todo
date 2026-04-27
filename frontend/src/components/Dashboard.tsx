import { useEffect, useState } from 'react';
import { Card, Table, Badge, Tag, Empty, Spin, Masonry, App, Button } from 'antd';
import {
  ArrowLeftOutlined,
  FileTextOutlined,
  PlayCircleOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  TagOutlined,
  ClockCircleOutlined,
  ThunderboltOutlined,
  DollarOutlined,
  BarChartOutlined,
} from '@ant-design/icons';
import { useApp } from '../hooks/useApp';
import { PieChart, PieChartLegend } from './PieChart';
import { TrendChart } from './dashboard/DashboardCharts';
import { AnimatedNumber } from './AnimatedNumber';
import * as db from '../utils/database';
import { getExecutorOption } from '../types';
import type { DashboardStats } from '../types';
import { formatRelativeTime } from '../utils/datetime';

const STATUS_COLORS: Record<string, string> = {
  pending: '#94a3b8',
  running: '#3b82f6',
  completed: '#22c55e',
  failed: '#ef4444',
};

const STATUS_LABELS: Record<string, string> = {
  pending: '待处理',
  running: '运行中',
  completed: '已完成',
  failed: '失败',
};

interface MiniStatProps {
  title: string;
  value: number;
  suffix?: string;
  prefix?: React.ReactNode;
  color: string;
  loading?: boolean;
  decimals?: number;
  chineseFormat?: boolean;
}

function MiniStat({ title, value, suffix, prefix, color, loading, decimals = 0, chineseFormat = false }: MiniStatProps) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
      <div
        style={{
          width: 40,
          height: 40,
          borderRadius: 10,
          backgroundColor: `${color}15`,
          color,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: 18,
          flexShrink: 0,
        }}
      >
        {prefix}
      </div>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginBottom: 2 }}>{title}</div>
        <div style={{ fontSize: 22, fontWeight: 700, color: 'var(--color-text)', lineHeight: 1.2 }}>
          {loading ? (
            <Spin size="small" />
          ) : (
            <>
              <AnimatedNumber value={value} duration={0.8} decimals={decimals} chineseFormat={chineseFormat} />
              {suffix && <span style={{ fontSize: 13, fontWeight: 500, marginLeft: 2 }}>{suffix}</span>}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function formatTokens(n: number): string {
  if (n >= 1_0000_0000) return (n / 1_0000_0000).toFixed(1) + '亿';
  if (n >= 1_0000) return (n / 1_0000).toFixed(1) + '万';
  return String(n);
}

function RichBarItem({
  label,
  value,
  color,
  max,
  detail,
}: {
  label: string;
  value: number;
  color: string;
  max: number;
  detail: React.ReactNode;
}) {
  const pct = max > 0 ? (value / max) * 100 : 0;
  return (
    <div style={{ marginBottom: 14 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 4 }}>
        <span
          style={{
            fontSize: 13,
            color: 'var(--color-text)',
            width: 80,
            textAlign: 'right',
            flexShrink: 0,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            fontWeight: 600,
          }}
          title={label}
        >
          {label}
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              width: `${pct}%`,
              minWidth: value > 0 ? 4 : 0,
              height: 18,
              borderRadius: 4,
              backgroundColor: color,
              transition: 'width 0.6s ease',
            }}
          />
        </div>
        <span
          style={{
            fontSize: 13,
            fontWeight: 700,
            color: 'var(--color-text)',
            width: 32,
            flexShrink: 0,
            textAlign: 'right',
          }}
        >
          {value}
        </span>
      </div>
      <div style={{ paddingLeft: 90, fontSize: 11, color: 'var(--color-text-tertiary)' }}>{detail}</div>
    </div>
  );
}


interface DashboardProps {
  onBack?: () => void;
}

export function Dashboard({ onBack }: DashboardProps) {
  const { state } = useApp();
  const { message } = App.useApp();
  const { todos, tags, runningTasks } = state;

  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        setLoading(true);
        const data = await db.getDashboardStats();
        if (!cancelled) setStats(data);
      } catch (err) {
        if (!cancelled) {
          message.error('加载统计数据失败');
          console.error(err);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const totalTodos = stats?.total_todos ?? todos.length;

  const statusSegments = [
    { value: stats?.pending_todos ?? 0, color: STATUS_COLORS.pending, label: STATUS_LABELS.pending },
    { value: stats?.running_todos ?? 0, color: STATUS_COLORS.running, label: STATUS_LABELS.running },
    { value: stats?.completed_todos ?? 0, color: STATUS_COLORS.completed, label: STATUS_LABELS.completed },
    { value: stats?.failed_todos ?? 0, color: STATUS_COLORS.failed, label: STATUS_LABELS.failed },
  ].filter((s) => s.value > 0);

  const executorData = stats?.executor_distribution ?? [];
  const executorMax = Math.max(...executorData.map((e) => e.count), 1);

  const tagData = stats?.tag_distribution ?? [];
  const tagMax = Math.max(...tagData.map((t) => t.count), 1);

  const tokenSegments = stats
    ? [
        { value: stats.total_input_tokens, color: '#3b82f6', label: '输入 Tokens' },
        { value: stats.total_output_tokens, color: '#22c55e', label: '输出 Tokens' },
        { value: stats.total_cache_read_tokens, color: '#f59e0b', label: '缓存读' },
        { value: stats.total_cache_creation_tokens, color: '#a78bfa', label: '缓存写' },
      ].filter((s) => s.value > 0)
    : [];

  const trendData = stats?.daily_executions ?? [];
  const runningList = Object.values(runningTasks);

  const successRate = stats && stats.total_executions > 0
    ? (stats.success_executions / stats.total_executions) * 100
    : 0;

  const recentColumns = [
    {
      title: '任务',
      dataIndex: 'todo_id',
      key: 'todo_id',
      render: (_: unknown, record: DashboardStats['recent_executions'][number]) => {
        const todo = todos.find((t) => t.id === record.todo_id);
        return <span style={{ fontWeight: 600 }}>{todo?.title ?? `任务 #${record.todo_id}`}</span>;
      },
    },
    {
      title: '执行器',
      dataIndex: 'executor',
      key: 'executor',
      width: 100,
      render: (v: string | null) => {
        if (!v) return <span>-</span>;
        const opt = getExecutorOption(v);
        return <Tag color={opt.color} style={{ fontWeight: 600 }}>{opt.label}</Tag>;
      },
    },
    {
      title: '触发',
      dataIndex: 'trigger_type',
      key: 'trigger_type',
      width: 70,
      render: (v: string) => (
        <Tag color={v === 'cron' ? '#8b5cf6' : '#6b7280'} style={{ fontSize: 10 }}>
          {v === 'cron' ? 'Cron' : '手动'}
        </Tag>
      ),
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 90,
      render: (v: string) => (
        <Badge
          status={v === 'success' ? 'success' : v === 'failed' ? 'error' : 'processing'}
          text={v === 'success' ? '成功' : v === 'failed' ? '失败' : '运行中'}
        />
      ),
    },
    {
      title: '时间',
      dataIndex: 'started_at',
      key: 'started_at',
      width: 140,
      render: (v: string) => <span style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }}>{formatRelativeTime(v)}</span>,
    },
  ];

  const panels: { key: string; render: () => React.ReactNode }[] = [];

  const ACTIVE_TASKS_MIN_HEIGHT = 148;

  panels.push({
    key: 'active-tasks',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><ThunderboltOutlined /><span>活跃任务</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: 0 }}
      >
        <div style={{ minHeight: ACTIVE_TASKS_MIN_HEIGHT, padding: '12px 16px' }}>
          {runningList.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10, maxHeight: ACTIVE_TASKS_MIN_HEIGHT - 24, overflow: 'auto' }}>
              {runningList.map((task) => {
                const opt = getExecutorOption(task.executor);
                return (
                  <div
                    key={task.taskId}
                    style={{
                      padding: '10px 14px',
                      borderRadius: 10,
                      background: 'var(--color-bg-elevated)',
                      border: '1px solid var(--color-border-secondary)',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      flexShrink: 0,
                    }}
                  >
                    <Badge status="processing" />
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontWeight: 600, fontSize: 13, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {task.todoTitle}
                      </div>
                      <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)' }}>
                        {opt.label} · {formatRelativeTime(task.startedAt)}
                      </div>
                    </div>
                    <Tag color={opt.color} style={{ fontSize: 11 }}>{opt.label}</Tag>
                  </div>
                );
              })}
            </div>
          ) : (
            <div style={{ textAlign: 'center', color: 'var(--color-text-secondary)', padding: '20px 0' }}>
              <div style={{ fontSize: 24, fontWeight: 700, marginBottom: 8, color: 'var(--color-text)' }}>
                Nothing Todo
              </div>
              <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 12, color: 'var(--color-text)' }}>
                无事可做
              </div>
              <div style={{ fontSize: 13, color: 'var(--color-text-tertiary)' }}>
                人类，你在干活，我无事可干
              </div>
            </div>
          )}
        </div>
      </Card>
    ),
  });

  panels.push({
    key: 'task-stats',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><FileTextOutlined /><span>任务概览</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 16 }}>
          <MiniStat title="总任务" value={totalTodos} prefix={<FileTextOutlined />} color="#0891b2" loading={loading && !stats} chineseFormat />
          <MiniStat title="运行中" value={stats?.running_todos ?? 0} prefix={<PlayCircleOutlined />} color="#3b82f6" loading={loading && !stats} />
          <MiniStat title="已完成" value={stats?.completed_todos ?? 0} prefix={<CheckCircleOutlined />} color="#22c55e" loading={loading && !stats} />
          <MiniStat title="失败" value={stats?.failed_todos ?? 0} prefix={<CloseCircleOutlined />} color="#ef4444" loading={loading && !stats} />
        </div>
      </Card>
    ),
  });

  panels.push({
    key: 'exec-stats',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><ThunderboltOutlined /><span>执行概览</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 16 }}>
          <MiniStat title="标签" value={stats?.total_tags ?? tags.length} prefix={<TagOutlined />} color="#8b5cf6" loading={loading && !stats} />
          <MiniStat title="调度任务" value={stats?.scheduled_todos ?? 0} prefix={<ClockCircleOutlined />} color="#f59e0b" loading={loading && !stats} />
          <MiniStat title="总执行" value={stats?.total_executions ?? 0} prefix={<ThunderboltOutlined />} color="#0d9488" loading={loading && !stats} chineseFormat />
          <MiniStat title="总花费" value={stats ? Math.round(stats.total_cost_usd * 10000) / 10000 : 0} suffix="$" prefix={<DollarOutlined />} color="#dc2626" loading={loading && !stats} decimals={4} />
        </div>
      </Card>
    ),
  });

  panels.push({
    key: 'status-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><BarChartOutlined /><span>任务状态分布</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        {statusSegments.length > 0 ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: 24, flexWrap: 'wrap' }}>
            <PieChart segments={statusSegments} size={140} centerText={String(totalTodos)} centerSubtext="总计" />
            <PieChartLegend segments={statusSegments} />
          </div>
        ) : (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无任务" />
        )}
      </Card>
    ),
  });

  panels.push({
    key: 'executor-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><BarChartOutlined /><span>执行器分布</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        {executorData.length > 0 ? (
          <div>
            {executorData.map((e) => {
              const opt = getExecutorOption(e.executor);
              const execRate = e.execution_count > 0 ? ((e.success_count / e.execution_count) * 100).toFixed(0) : '0';
              return (
                <RichBarItem
                  key={e.executor}
                  label={opt.label}
                  value={e.count}
                  color={opt.color}
                  max={executorMax}
                  detail={
                    <span>
                      执行 <strong style={{ color: 'var(--color-text)' }}><AnimatedNumber value={e.execution_count} duration={0.6} /></strong> 次
                      <span style={{ margin: '0 6px', color: 'var(--color-border)' }}>|</span>
                      成功率 <strong style={{ color: '#22c55e' }}>{execRate}%</strong>
                      {e.total_cost_usd > 0 && (
                        <>
                          <span style={{ margin: '0 6px', color: 'var(--color-border)' }}>|</span>
                          <span style={{ color: '#f59e0b', fontWeight: 600 }}>${e.total_cost_usd.toFixed(2)}</span>
                        </>
                      )}
                    </span>
                  }
                />
              );
            })}
          </div>
        ) : (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无数据" />
        )}
      </Card>
    ),
  });

  panels.push({
    key: 'tag-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><TagOutlined /><span>标签分布</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        {tagData.length > 0 ? (
          <div>
            {tagData.map((t) => {
              const execRate = t.execution_count > 0 ? ((t.success_count / t.execution_count) * 100).toFixed(0) : '0';
              return (
                <RichBarItem
                  key={t.tag_id}
                  label={t.tag_name}
                  value={t.count}
                  color={t.tag_color}
                  max={tagMax}
                  detail={
                    <span>
                      执行 <strong style={{ color: 'var(--color-text)' }}><AnimatedNumber value={t.execution_count} duration={0.6} /></strong> 次
                      <span style={{ margin: '0 6px', color: 'var(--color-border)' }}>|</span>
                      成功率 <strong style={{ color: '#22c55e' }}>{execRate}%</strong>
                      {t.total_cost_usd > 0 && (
                        <>
                          <span style={{ margin: '0 6px', color: 'var(--color-border)' }}>|</span>
                          <span style={{ color: '#f59e0b', fontWeight: 600 }}>${t.total_cost_usd.toFixed(2)}</span>
                        </>
                      )}
                    </span>
                  }
                />
              );
            })}
          </div>
        ) : (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无标签数据" />
        )}
      </Card>
    ),
  });

  panels.push({
    key: 'token-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><ThunderboltOutlined /><span>Token 消耗</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        {tokenSegments.length > 0 ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: 24, flexWrap: 'wrap' }}>
            <PieChart
              segments={tokenSegments}
              size={140}
              centerText={
                stats
                  ? formatTokens(
                      stats.total_input_tokens +
                      stats.total_output_tokens +
                      stats.total_cache_read_tokens +
                      stats.total_cache_creation_tokens
                    )
                  : '0'
              }
              centerSubtext="Tokens"
            />
            <PieChartLegend segments={tokenSegments} chineseFormat />
          </div>
        ) : (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无 Token 数据" />
        )}
      </Card>
    ),
  });

  panels.push({
    key: 'trend-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><BarChartOutlined /><span>执行趋势（近30天）</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        <TrendChart data={trendData} height={180} />
      </Card>
    ),
  });

  const modelData = stats?.model_distribution ?? [];
  const modelMax = Math.max(...modelData.map((m) => m.total_cost_usd), 0.01);

  panels.push({
    key: 'model-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><BarChartOutlined /><span>模型分布</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        {modelData.length > 0 ? (
          <div>
            {modelData.map((m) => {
              const modelRate = m.execution_count > 0 ? ((m.success_count / m.execution_count) * 100).toFixed(0) : '0';
              return (
                <RichBarItem
                  key={m.model}
                  label={m.model.length > 10 ? m.model.slice(0, 10) + '...' : m.model}
                  value={m.total_cost_usd}
                  color="#8b5cf6"
                  max={modelMax}
                  detail={
                    <span>
                      执行 <strong style={{ color: 'var(--color-text)' }}><AnimatedNumber value={m.execution_count} duration={0.6} /></strong> 次
                      <span style={{ margin: '0 6px', color: 'var(--color-border)' }}>|</span>
                      输入 <strong style={{ color: '#3b82f6' }}>{formatTokens(m.total_input_tokens)}</strong>
                      <span style={{ margin: '0 6px', color: 'var(--color-border)' }}>|</span>
                      输出 <strong style={{ color: '#22c55e' }}>{formatTokens(m.total_output_tokens)}</strong>
                      <span style={{ margin: '0 6px', color: 'var(--color-border)' }}>|</span>
                      成功率 <strong style={{ color: '#22c55e' }}>{modelRate}%</strong>
                      {m.total_cost_usd > 0 && (
                        <>
                          <span style={{ margin: '0 6px', color: 'var(--color-border)' }}>|</span>
                          <span style={{ color: '#f59e0b', fontWeight: 600 }}>${m.total_cost_usd.toFixed(2)}</span>
                        </>
                      )}
                    </span>
                  }
                />
              );
            })}
          </div>
        ) : (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无模型数据" />
        )}
      </Card>
    ),
  });

  panels.push({
    key: 'token-trend-chart',
    render: () => {
      const tokenTrendData = stats?.daily_token_stats ?? [];
      const maxToken = Math.max(...tokenTrendData.map(d => d.input_tokens + d.output_tokens), 1);

      const svg = tokenTrendData.length > 0 ? (
        <svg width="100%" height={180} viewBox="0 0 600 180" style={{ overflow: 'visible' }}>
          {(() => {
            const w = 600;
            const h = 180;
            const padL = 45;
            const padR = 12;
            const padB = 28;
            const padT = 12;
            const chartW = w - padL - padR;
            const chartH = h - padT - padB;
            const barW = tokenTrendData.length > 0 ? chartW / tokenTrendData.length * 0.7 : 0;
            const gap = tokenTrendData.length > 0 ? chartW / tokenTrendData.length * 0.3 : 0;

            const yTicks = [0, maxToken * 0.5, maxToken];

            return (
              <>
                {yTicks.map((t, i) => {
                  const y = padT + chartH - (t / maxToken) * chartH;
                  return (
                    <g key={i}>
                      <line x1={padL} y1={y} x2={w - padR} y2={y} stroke="#e2e8f0" strokeWidth={1} />
                      <text x={padL - 6} y={y + 4} textAnchor="end" fontSize={10} fill="#94a3b8">
                        {t >= 10000 ? `${(t/10000).toFixed(0)}w` : t}
                      </text>
                    </g>
                  );
                })}
                {tokenTrendData.map((d, i) => {
                  const x = padL + i * (barW + gap) + gap / 2;
                  const inputH = d.input_tokens / maxToken * chartH;
                  const outputH = d.output_tokens / maxToken * chartH;
                  return (
                    <g key={i}>
                      <rect
                        x={x}
                        y={padT + chartH - inputH}
                        width={barW}
                        height={inputH}
                        fill="#3b82f6"
                        rx={2}
                      />
                      <rect
                        x={x}
                        y={padT + chartH - inputH - outputH}
                        width={barW}
                        height={outputH}
                        fill="#22c55e"
                        rx={2}
                      />
                      <text
                        x={x + barW / 2}
                        y={h - 6}
                        textAnchor="middle"
                        fontSize={9}
                        fill="#94a3b8"
                        transform={tokenTrendData.length > 14 ? `rotate(-35, ${x + barW / 2}, ${h - 6})` : undefined}
                      >
                        {d.date.slice(5)}
                      </text>
                    </g>
                  );
                })}
              </>
            );
          })()}
        </svg>
      ) : null;

      return (
        <Card
          title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><BarChartOutlined /><span>Token 趋势（近30天）</span></div>}
          style={{ borderRadius: 12 }}
          bodyStyle={{ padding: '16px 20px' }}
        >
          {tokenTrendData.length > 0 ? (
            <div style={{ width: '100%' }}>
              <div style={{ display: 'flex', gap: 16, marginBottom: 8, justifyContent: 'flex-end' }}>
                <span style={{ fontSize: 11, color: '#3b82f6', display: 'flex', alignItems: 'center', gap: 4 }}>
                  <span style={{ width: 8, height: 8, borderRadius: 2, background: '#3b82f6' }} />
                  输入
                </span>
                <span style={{ fontSize: 11, color: '#22c55e', display: 'flex', alignItems: 'center', gap: 4 }}>
                  <span style={{ width: 8, height: 8, borderRadius: 2, background: '#22c55e' }} />
                  输出
                </span>
              </div>
              {svg}
            </div>
          ) : (
            <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无 Token 趋势数据" />
          )}
        </Card>
      );
    },
  });

  panels.push({
    key: 'overview-card',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><ThunderboltOutlined /><span>执行概览</span></div>}
        style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
              <span style={{ fontSize: 13, color: 'var(--color-text-secondary)' }}>成功率</span>
              <span style={{ fontSize: 13, fontWeight: 700, color: '#22c55e' }}>{successRate.toFixed(1)}%</span>
            </div>
            <div style={{ height: 8, borderRadius: 4, background: '#e2e8f0', overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${successRate}%`, borderRadius: 4, background: '#22c55e', transition: 'width 0.8s ease' }} />
            </div>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', padding: '10px 0', borderBottom: '1px solid var(--color-border-secondary)' }}>
            <span style={{ fontSize: 13, color: 'var(--color-text-secondary)' }}>成功执行</span>
            <span style={{ fontSize: 14, fontWeight: 700, color: '#22c55e' }}><AnimatedNumber value={stats?.success_executions ?? 0} duration={0.8} chineseFormat /></span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', padding: '10px 0', borderBottom: '1px solid var(--color-border-secondary)' }}>
            <span style={{ fontSize: 13, color: 'var(--color-text-secondary)' }}>失败执行</span>
            <span style={{ fontSize: 14, fontWeight: 700, color: '#ef4444' }}><AnimatedNumber value={stats?.failed_executions ?? 0} duration={0.8} chineseFormat /></span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', padding: '10px 0', borderBottom: '1px solid var(--color-border-secondary)' }}>
            <span style={{ fontSize: 13, color: 'var(--color-text-secondary)' }}>平均耗时</span>
            <span style={{ fontSize: 14, fontWeight: 700, color: 'var(--color-text)' }}>
              {stats && stats.avg_duration_ms > 0 ? `${(stats.avg_duration_ms / 1000).toFixed(2)}s` : '-'}
            </span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', padding: '10px 0' }}>
            <span style={{ fontSize: 13, color: 'var(--color-text-secondary)' }}>总花费</span>
            <span style={{ fontSize: 14, fontWeight: 700, color: '#f59e0b' }}>${stats ? stats.total_cost_usd.toFixed(4) : '0.0000'}</span>
          </div>
        </div>
      </Card>
    ),
  });

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '16px 20px', background: 'var(--color-bg-layout)' }}>
      {onBack && (
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={onBack}
          style={{ marginBottom: 12, marginLeft: -4 }}
        >
          返回任务列表
        </Button>
      )}
      <Masonry
        columns={{ xs: 1, sm: 1, md: 2, lg: 2, xl: 3 }}
        gutter={[16, 16]}
        items={panels.map(p => ({ key: p.key, data: p }))}
        itemRender={(item) => item.data.render()}
        fresh
      />
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><ThunderboltOutlined /><span>最近执行记录</span></div>}
        style={{ borderRadius: 12, marginTop: 16 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        {stats && stats.recent_executions.length > 0 ? (
          <Table columns={recentColumns} dataSource={stats.recent_executions} rowKey="id" pagination={false} size="small" scroll={{ x: 'max-content' }} />
        ) : (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无执行记录" />
        )}
      </Card>
    </div>
  );
}
