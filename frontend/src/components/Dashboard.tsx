import { useEffect, useState } from 'react';
import { Card, Table, Badge, Tag, Empty, Masonry, App, Button } from 'antd';
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
    <div style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 14px', borderRadius: 10, background: 'var(--color-fill-quaternary)', transition: 'background 0.2s' }}>
      <div
        style={{
          width: 40,
          height: 40,
          borderRadius: 10,
          backgroundColor: `${color}18`,
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
          <AnimatedNumber value={loading ? 0 : value} duration={0.8} decimals={decimals} chineseFormat={chineseFormat} />
          {suffix && <span style={{ fontSize: 13, fontWeight: 500, marginLeft: 2 }}>{suffix}</span>}
        </div>
      </div>
    </div>
  );
}

function CompactRow({ name, value, sub, color, barPct }: {
  name: string; value: React.ReactNode; sub: React.ReactNode; color: string; barPct: number;
}) {
  return (
    <div style={{ padding: '10px 0', borderBottom: '1px solid var(--color-border-secondary)' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 6 }}>
        <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-text)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', marginRight: 12 }} title={name}>{name}</span>
        {value}
      </div>
      <div style={{ height: 4, borderRadius: 2, background: 'var(--color-fill-quaternary)', marginBottom: 6 }}>
        <div style={{ height: '100%', width: `${Math.max(barPct, 0)}%`, minWidth: barPct > 0 ? 4 : 0, borderRadius: 2, background: color, transition: 'width 0.6s ease' }} />
      </div>
      <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)' }}>{sub}</div>
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
      } catch {
        if (!cancelled) {
          message.error('加载统计数据失败');
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
        className="dashboard-card" style={{ borderRadius: 12 }}
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
            <div style={{ color: 'var(--color-text-secondary)', padding: '20px 0' }}>
              <div style={{ textAlign: 'center' }}>
                <div style={{ fontSize: 24, fontWeight: 700, marginBottom: 8, color: 'var(--color-text)' }}>
                  everything is todo
                </div>
                <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 12, color: 'var(--color-text)' }}>
                  but now nothing todo
                </div>
                <div style={{ fontSize: 13, color: 'var(--color-text-tertiary)', marginBottom: 4 }}>
                  万事皆待办，此刻无事可干。
                </div>
              </div>
              <div style={{ textAlign: 'right', fontSize: 12, color: 'var(--color-text-tertiary)' }}>
                💭 人类，你在忙，而我无事可干。
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
        className="dashboard-card" style={{ borderRadius: 12 }}
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
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 16 }}>
          <MiniStat title="标签" value={stats?.total_tags ?? tags.length} prefix={<TagOutlined />} color="#8b5cf6" loading={loading && !stats} />
          <MiniStat title="定时" value={stats?.scheduled_todos ?? 0} prefix={<ClockCircleOutlined />} color="#f59e0b" loading={loading && !stats} />
          <MiniStat title="总执行" value={stats?.total_executions ?? 0} prefix={<ThunderboltOutlined />} color="#0d9488" loading={loading && !stats} chineseFormat />
          <MiniStat title="总花费" value={stats ? Math.round(stats.total_cost_usd) : 0} suffix="$" prefix={<DollarOutlined />} color="#dc2626" loading={loading && !stats} />
        </div>
      </Card>
    ),
  });

  panels.push({
    key: 'status-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><BarChartOutlined /><span>任务状态分布</span></div>}
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        {statusSegments.length > 0 ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: 24, flexWrap: 'wrap' }}>
            <PieChart segments={statusSegments} size={140} centerText={<AnimatedNumber value={totalTodos} duration={1.2} chineseFormat />} centerSubtext="总计" />
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
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '8px 16px' }}
      >
        {executorData.length > 0 ? (
          executorData.map((e) => {
            const opt = getExecutorOption(e.executor);
            const execRate = e.execution_count > 0 ? ((e.success_count / e.execution_count) * 100).toFixed(0) : '0';
            return (
              <CompactRow
                key={e.executor}
                name={opt.label}
                value={<span style={{ fontSize: 18, fontWeight: 700, color: opt.color }}>{e.count}</span>}
                color={opt.color}
                barPct={(e.count / executorMax) * 100}
                sub={
                  <span>
                    执行 <strong style={{ color: 'var(--color-text)' }}>{e.execution_count}</strong> 次
                    <span style={{ margin: '0 6px' }}>·</span>
                    成功率 <strong style={{ color: '#22c55e' }}>{execRate}%</strong>
                    {e.total_cost_usd > 0 && (
                      <>
                        <span style={{ margin: '0 6px' }}>·</span>
                        <span style={{ color: '#f59e0b', fontWeight: 600 }}>${Math.round(e.total_cost_usd)}</span>
                      </>
                    )}
                  </span>
                }
              />
            );
          })
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
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '8px 16px' }}
      >
        {tagData.length > 0 ? (
          tagData.map((t) => {
            const execRate = t.execution_count > 0 ? ((t.success_count / t.execution_count) * 100).toFixed(0) : '0';
            return (
              <CompactRow
                key={t.tag_id}
                name={t.tag_name}
                value={<span style={{ fontSize: 18, fontWeight: 700, color: t.tag_color }}>{t.count}</span>}
                color={t.tag_color}
                barPct={(t.count / tagMax) * 100}
                sub={
                  <span>
                    执行 <strong style={{ color: 'var(--color-text)' }}>{t.execution_count}</strong> 次
                    <span style={{ margin: '0 6px' }}>·</span>
                    成功率 <strong style={{ color: '#22c55e' }}>{execRate}%</strong>
                    {t.total_cost_usd > 0 && (
                      <>
                        <span style={{ margin: '0 6px' }}>·</span>
                        <span style={{ color: '#f59e0b', fontWeight: 600 }}>${Math.round(t.total_cost_usd)}</span>
                      </>
                    )}
                  </span>
                }
              />
            );
          })
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
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        {tokenSegments.length > 0 ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: 24, flexWrap: 'wrap' }}>
            <PieChart
              segments={tokenSegments}
              size={140}
              centerText={
                stats
                  ? <AnimatedNumber value={stats.total_input_tokens + stats.total_output_tokens + stats.total_cache_read_tokens + stats.total_cache_creation_tokens} duration={1.2} chineseFormat />
                  : <AnimatedNumber value={0} duration={1.2} chineseFormat />
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
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        <TrendChart data={trendData} height={180} />
      </Card>
    ),
  });

  const modelData = stats?.model_distribution ?? [];
  const modelCountMax = Math.max(...modelData.map((m) => m.count), 1);
  const modelTokenMax = Math.max(...modelData.map((m) => m.total_input_tokens + m.total_output_tokens), 1);

  const MODEL_COLORS = ['#8b5cf6', '#3b82f6', '#22c55e', '#f59e0b', '#ef4444', '#0891b2', '#ec4899', '#6366f1'];

  panels.push({
    key: 'model-task-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><BarChartOutlined /><span>模型任务分布</span></div>}
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '8px 16px' }}
      >
        {modelData.length > 0 ? (
          modelData.map((m, i) => {
            const rate = m.execution_count > 0 ? ((m.success_count / m.execution_count) * 100).toFixed(0) : '0';
            return (
              <CompactRow
                key={m.model}
                name={m.model}
                value={<span style={{ fontSize: 18, fontWeight: 700, color: MODEL_COLORS[i % MODEL_COLORS.length] }}>{m.count}</span>}
                color={MODEL_COLORS[i % MODEL_COLORS.length]}
                barPct={(m.count / modelCountMax) * 100}
                sub={
                  <span>
                    执行 <strong style={{ color: 'var(--color-text)' }}>{m.execution_count}</strong> 次
                    <span style={{ margin: '0 6px' }}>·</span>
                    成功率 <strong style={{ color: '#22c55e' }}>{rate}%</strong>
                  </span>
                }
              />
            );
          })
        ) : (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无模型数据" />
        )}
      </Card>
    ),
  });

  panels.push({
    key: 'model-token-chart',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><ThunderboltOutlined /><span>模型推理统计</span></div>}
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '8px 16px' }}
      >
        {modelData.length > 0 ? (
          modelData.map((m, i) => {
            const outputRate = m.total_input_tokens > 0 ? (m.total_output_tokens / m.total_input_tokens) * 100 : 0;
            const costDisplay = m.total_cost_usd < 10000 ? `$${m.total_cost_usd.toFixed(2)}` : `$${(m.total_cost_usd / 10000).toFixed(2)}万`;
            return (
              <CompactRow
                key={m.model}
                name={m.model}
                value={<span style={{ fontSize: 16, fontWeight: 700, color: MODEL_COLORS[i % MODEL_COLORS.length] }}>{(m.total_input_tokens / 10000).toFixed(1)}万</span>}
                color={MODEL_COLORS[i % MODEL_COLORS.length]}
                barPct={(m.total_input_tokens / modelTokenMax) * 100}
                sub={
                  <span>
                    推理输入 <strong style={{ color: '#3b82f6' }}>{(m.total_input_tokens / 10000).toFixed(1)}万</strong>
                    <span style={{ margin: '0 4px' }}>·</span>
                    成本 <strong style={{ color: '#f59e0b' }}>{costDisplay}</strong>
                    <span style={{ margin: '0 4px' }}>·</span>
                    输出率 <strong style={{ color: '#22c55e' }}>{outputRate.toFixed(1)}%</strong>
                  </span>
                }
              />
            );
          })
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

            const yTicks = [0, maxToken * 0.5, maxToken];

            const points = tokenTrendData.map((d, i) => {
              const x = padL + (i / Math.max(tokenTrendData.length - 1, 1)) * chartW;
              const inputY = padT + chartH - (d.input_tokens / maxToken) * chartH;
              const outputY = padT + chartH - (d.output_tokens / maxToken) * chartH;
              return { x, inputY, outputY, date: d.date };
            });

            const inputPath = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.inputY}`).join(' ');
            const outputPath = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.outputY}`).join(' ');

            return (
              <>
                {yTicks.map((t, i) => {
                  const y = padT + chartH - (t / maxToken) * chartH;
                  return (
                    <g key={i}>
                      <line x1={padL} y1={y} x2={w - padR} y2={y} stroke="var(--color-border-secondary)" strokeWidth={1} />
                      <text x={padL - 6} y={y + 4} textAnchor="end" fontSize={10} fill="var(--color-text-tertiary)">
                        {t >= 10000 ? `${(t/10000).toFixed(0)}w` : t}
                      </text>
                    </g>
                  );
                })}
                <path d={inputPath} fill="none" stroke="#3b82f6" strokeWidth={2} strokeLinejoin="round" />
                <path d={outputPath} fill="none" stroke="#22c55e" strokeWidth={2} strokeLinejoin="round" />
                {points.map((p, i) => (
                  <g key={i}>
                    <circle cx={p.x} cy={p.inputY} r={3} fill="#3b82f6" />
                    <circle cx={p.x} cy={p.outputY} r={3} fill="#22c55e" />
                    <text
                      x={p.x}
                      y={h - 6}
                      textAnchor="middle"
                      fontSize={9}
                      fill="var(--color-text-tertiary)"
                      transform={tokenTrendData.length > 14 ? `rotate(-35, ${p.x}, ${h - 6})` : undefined}
                    >
                      {p.date.slice(5)}
                    </text>
                  </g>
                ))}
              </>
            );
          })()}
        </svg>
      ) : null;

      return (
        <Card
          title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><BarChartOutlined /><span>Token 趋势（近30天）</span></div>}
          className="dashboard-card" style={{ borderRadius: 12 }}
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
    key: 'inference-stats',
    render: () => {
      const totalInput = stats?.total_input_tokens ?? 0;
      const totalOutput = stats?.total_output_tokens ?? 0;
      const totalCost = stats?.total_cost_usd ?? 0;
      const outputRate = totalInput > 0 ? (totalOutput / totalInput) * 100 : 0;

      return (
        <Card
          title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><ThunderboltOutlined /><span>推理统计</span></div>}
          className="dashboard-card" style={{ borderRadius: 12 }}
          bodyStyle={{ padding: '16px 20px' }}
        >
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12 }}>
            <div style={{ padding: '12px 14px', borderRadius: 10, background: '#3b82f610', textAlign: 'center' }}>
              <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', marginBottom: 4 }}>推理输入</div>
              <div style={{ fontSize: 20, fontWeight: 700, color: '#3b82f6' }}>
                <AnimatedNumber value={loading ? 0 : totalInput / 10000} duration={1.2} decimals={2} suffix="万" />
              </div>
            </div>
            <div style={{ padding: '12px 14px', borderRadius: 10, background: '#f59e0b10', textAlign: 'center' }}>
              <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', marginBottom: 4 }}>成本</div>
              <div style={{ fontSize: 20, fontWeight: 700, color: '#f59e0b' }}>
                <AnimatedNumber value={loading ? 0 : totalCost} duration={1.2} prefix="$" decimals={2} />
              </div>
            </div>
            <div style={{ padding: '12px 14px', borderRadius: 10, background: '#22c55e10', textAlign: 'center' }}>
              <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', marginBottom: 4 }}>输出率</div>
              <div style={{ fontSize: 20, fontWeight: 700, color: '#22c55e' }}>
                <AnimatedNumber value={loading ? 0 : outputRate} duration={1.2} decimals={1} suffix="%" />
              </div>
            </div>
          </div>
        </Card>
      );
    },
  });

  panels.push({
    key: 'overview-card',
    render: () => (
      <Card
        title={<div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><ThunderboltOutlined /><span>执行概览</span></div>}
        className="dashboard-card" style={{ borderRadius: 12 }}
        bodyStyle={{ padding: '16px 20px' }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
              <span style={{ fontSize: 13, color: 'var(--color-text-secondary)' }}>成功率</span>
              <span style={{ fontSize: 15, fontWeight: 700, color: '#22c55e' }}><AnimatedNumber value={successRate} duration={1.2} decimals={1} suffix="%" /></span>
            </div>
            <div style={{ height: 6, borderRadius: 3, background: 'var(--color-fill-quaternary)', overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${successRate}%`, borderRadius: 3, background: 'linear-gradient(90deg, #22c55e, #4ade80)', transition: 'width 0.8s ease' }} />
            </div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div style={{ padding: '10px 14px', borderRadius: 10, background: '#22c55e10' }}>
              <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', marginBottom: 2 }}>成功执行</div>
              <div style={{ fontSize: 18, fontWeight: 700, color: '#22c55e' }}><AnimatedNumber value={stats?.success_executions ?? 0} duration={0.8} chineseFormat /></div>
            </div>
            <div style={{ padding: '10px 14px', borderRadius: 10, background: '#ef444410' }}>
              <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', marginBottom: 2 }}>失败执行</div>
              <div style={{ fontSize: 18, fontWeight: 700, color: '#ef4444' }}><AnimatedNumber value={stats?.failed_executions ?? 0} duration={0.8} chineseFormat /></div>
            </div>
            <div style={{ padding: '10px 14px', borderRadius: 10, background: 'var(--color-fill-quaternary)' }}>
              <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', marginBottom: 2 }}>平均耗时</div>
              <div style={{ fontSize: 18, fontWeight: 700, color: 'var(--color-text)' }}>
                {stats && stats.avg_duration_ms > 0 ? <AnimatedNumber value={stats.avg_duration_ms / 1000} duration={1.2} decimals={1} suffix="s" /> : '-'}
              </div>
            </div>
            <div style={{ padding: '10px 14px', borderRadius: 10, background: '#f59e0b10' }}>
              <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', marginBottom: 2 }}>总花费</div>
              <div style={{ fontSize: 18, fontWeight: 700, color: '#f59e0b' }}><AnimatedNumber value={stats ? Math.round(stats.total_cost_usd) : 0} duration={1.2} prefix="$" /></div>
            </div>
          </div>
        </div>
      </Card>
    ),
  });

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '16px 20px', background: 'var(--color-bg-layout)' }}>
      <style>{`
        .dashboard-card { transition: border-color 0.2s, box-shadow 0.2s; }
        .dashboard-card:hover { border-color: var(--color-border); box-shadow: 0 2px 12px rgba(0,0,0,0.08); }
      `}</style>
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
