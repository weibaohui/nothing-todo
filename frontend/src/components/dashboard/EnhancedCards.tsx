import { Empty } from 'antd';
import { ArrowUpOutlined, ArrowDownOutlined, CrownOutlined } from '@ant-design/icons';
import { AnimatedNumber } from '@/components/AnimatedNumber';

interface MetricCardProps {
  title: string;
  value: number;
  suffix?: string;
  prefix?: React.ReactNode;
  change?: number;
  changeLabel?: string;
  color: string;
  loading?: boolean;
  decimals?: number;
  chineseFormat?: boolean;
}

export function MetricCard({
  title,
  value,
  suffix,
  prefix,
  change,
  changeLabel,
  color,
  loading = false,
  decimals = 0,
  chineseFormat = false,
}: MetricCardProps) {
  const isPositive = change !== undefined && change > 0;
  const isNegative = change !== undefined && change < 0;
  const changeColor = isPositive ? '#22c55e' : isNegative ? '#ef4444' : 'var(--color-text-tertiary)';
  const ChangeIcon = isPositive ? ArrowUpOutlined : isNegative ? ArrowDownOutlined : null;

  return (
    <div
      style={{
        padding: '16px 18px',
        borderRadius: 12,
        background: 'var(--color-fill-quaternary)',
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>{title}</span>
        {prefix && (
          <div
            style={{
              width: 28,
              height: 28,
              borderRadius: 8,
              backgroundColor: `${color}18`,
              color,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: 14,
            }}
          >
            {prefix}
          </div>
        )}
      </div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 6 }}>
        <span style={{ fontSize: 28, fontWeight: 700, color: 'var(--color-text)', lineHeight: 1.2 }}>
          <AnimatedNumber value={loading ? 0 : value} duration={0.8} decimals={decimals} chineseFormat={chineseFormat} />
        </span>
        {suffix && (
          <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--color-text-secondary)' }}>{suffix}</span>
        )}
      </div>
      {change !== undefined && change !== null && change !== 0 && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          {ChangeIcon && <ChangeIcon style={{ fontSize: 10, color: changeColor }} />}
          <span style={{ fontSize: 11, color: changeColor, fontWeight: 600 }}>
            {Math.abs(change).toFixed(1)}%
          </span>
          {changeLabel && (
            <span style={{ fontSize: 11, color: 'var(--color-text-tertiary)' }}>{changeLabel}</span>
          )}
        </div>
      )}
    </div>
  );
}

interface LeaderboardItem {
  rank: number;
  name: string;
  avatar?: string;
  tokens: number;
  sessions: number;
  change?: number;
}

interface LeaderboardProps {
  data: LeaderboardItem[];
  maxTokens?: number;
  loading?: boolean;
}

const RANK_COLORS = ['#f59e0b', '#94a3b8', '#cd7f32'];

export function Leaderboard({ data, maxTokens, loading: _loading = false }: LeaderboardProps) {
  const max = maxTokens ?? Math.max(1, ...data.map(d => d.tokens ?? 0));

  if (data.length === 0) {
    return <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无排行数据" />;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {data.map((item) => {
        const rankColor = item.rank <= 3 ? RANK_COLORS[item.rank - 1] : 'var(--color-text-tertiary)';
        const isTop3 = item.rank <= 3;
        return (
          <div
            key={item.rank}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '10px 14px',
              borderRadius: 10,
              background: isTop3 ? `${rankColor}08` : 'var(--color-fill-quaternary)',
              border: isTop3 ? `1px solid ${rankColor}30` : '1px solid transparent',
            }}
          >
            <div
              style={{
                width: 28,
                height: 28,
                borderRadius: 8,
                background: isTop3 ? `${rankColor}20` : 'var(--color-fill-elevated)',
                color: rankColor,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontSize: 12,
                fontWeight: 700,
                flexShrink: 0,
              }}
            >
              {isTop3 ? <CrownOutlined /> : `#${item.rank}`}
            </div>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-text)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {item.name}
              </div>
              <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)' }}>
                {item.sessions} sessions
              </div>
            </div>
            <div style={{ textAlign: 'right' }}>
              <div style={{ fontSize: 14, fontWeight: 700, color: 'var(--color-text)' }}>
                {item.tokens >= 10000 ? `${(item.tokens / 10000).toFixed(1)}w` : (item.tokens ?? 0)}
              </div>
              {item.change != null && (
                <div style={{ fontSize: 10, color: (item.change ?? 0) > 0 ? '#22c55e' : '#ef4444' }}>
                  {(item.change ?? 0) > 0 ? '+' : ''}{(item.change ?? 0).toFixed(1)}%
                </div>
              )}
            </div>
            <div
              style={{
                width: 40,
                height: 4,
                borderRadius: 2,
                background: 'var(--color-fill-quaternary)',
                overflow: 'hidden',
                flexShrink: 0,
              }}
            >
              <div
                style={{
                  height: '100%',
                  width: `${(item.tokens / max) * 100}%`,
                  minWidth: item.tokens > 0 ? 4 : 0,
                  borderRadius: 2,
                  background: rankColor,
                  transition: 'width 0.6s ease',
                }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

interface HighlightStatProps {
  label: string;
  value: number | string;
  subLabel?: string;
  subValue?: string;
  color: string;
  icon?: React.ReactNode;
}

export function HighlightStat({ label, value, subLabel, subValue, color, icon }: HighlightStatProps) {
  const displayValue = value ?? '-';
  return (
    <div
      style={{
        padding: '14px 16px',
        borderRadius: 10,
        background: `${color}10`,
        border: `1px solid ${color}25`,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 8 }}>
        {icon && <span style={{ color, fontSize: 14 }}>{icon}</span>}
        <span style={{ fontSize: 11, color: 'var(--color-text-secondary)' }}>{label}</span>
      </div>
      <div style={{ fontSize: 22, fontWeight: 700, color, marginBottom: 4 }}>
        {typeof displayValue === 'number' ? (
          <AnimatedNumber value={displayValue} duration={1} />
        ) : displayValue}
      </div>
      {subLabel && (
        <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)' }}>
          {subLabel}
          {subValue && <span style={{ color, fontWeight: 600 }}> {subValue}</span>}
        </div>
      )}
    </div>
  );
}
