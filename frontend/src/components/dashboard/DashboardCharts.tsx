import { useMemo } from 'react';

interface BarItem {
  label: string;
  value: number;
  color: string;
}

interface HorizontalBarChartProps {
  data: BarItem[];
  maxValue?: number;
  barHeight?: number;
  showValues?: boolean;
}

export function HorizontalBarChart({
  data,
  maxValue,
  barHeight = 20,
  showValues = true,
}: HorizontalBarChartProps) {
  const max = maxValue ?? Math.max(...data.map((d) => d.value), 1);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      {data.map((item) => {
        const pct = max > 0 ? (item.value / max) * 100 : 0;
        return (
          <div key={item.label} style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <span
              style={{
                fontSize: 12,
                color: 'var(--color-text-secondary)',
                width: 80,
                textAlign: 'right',
                flexShrink: 0,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
              title={item.label}
            >
              {item.label}
            </span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div
                style={{
                  width: `${pct}%`,
                  minWidth: item.value > 0 ? 4 : 0,
                  height: barHeight,
                  borderRadius: 4,
                  backgroundColor: item.color,
                  transition: 'width 0.6s ease',
                }}
              />
            </div>
            {showValues && (
              <span
                style={{
                  fontSize: 12,
                  fontWeight: 600,
                  color: 'var(--color-text)',
                  width: 32,
                  flexShrink: 0,
                  textAlign: 'right',
                }}
              >
                {item.value}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

interface TrendPoint {
  date: string;
  success: number;
  failed: number;
}

interface TrendChartProps {
  data: TrendPoint[];
  height?: number;
}

export function TrendChart({ data, height = 160 }: TrendChartProps) {
  const svg = useMemo(() => {
    if (data.length === 0) return null;

    const w = 600;
    const h = height;
    const padL = 40;
    const padR = 12;
    const padB = 28;
    const padT = 12;
    const chartW = w - padL - padR;
    const chartH = h - padT - padB;

    const maxVal = Math.max(...data.map((d) => d.success + d.failed), 1);
    const barW = data.length > 0 ? chartW / data.length * 0.7 : 0;
    const gap = data.length > 0 ? chartW / data.length * 0.3 : 0;

    const bars = data.map((d, i) => {
      const x = padL + i * (barW + gap) + gap / 2;
      const totalH = (d.success + d.failed) / maxVal * chartH;
      const succH = d.success / maxVal * chartH;
      const failH = d.failed / maxVal * chartH;
      return { x, totalH, succH, failH, date: d.date, success: d.success, failed: d.failed };
    });

    const yTicks = [0, maxVal * 0.5, maxVal];

    return (
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} style={{ overflow: 'visible' }}>
        {/* Y axis lines */}
        {yTicks.map((t, i) => {
          const y = padT + chartH - (t / maxVal) * chartH;
          return (
            <g key={i}>
              <line x1={padL} y1={y} x2={w - padR} y2={y} stroke="var(--color-border)" strokeWidth={1} />
              <text x={padL - 6} y={y + 4} textAnchor="end" fontSize={10} fill="var(--color-text-tertiary)">
                {Math.round(t)}
              </text>
            </g>
          );
        })}

        {/* Bars */}
        {bars.map((b, i) => (
          <g key={i}>
            {/* Success portion */}
            {b.succH > 0 && (
              <rect
                x={b.x}
                y={padT + chartH - b.succH - (b.failH > 0 ? b.failH : 0)}
                width={barW}
                height={b.succH}
                fill="var(--color-success)"
                rx={2}
              />
            )}
            {/* Failed portion */}
            {b.failH > 0 && (
              <rect
                x={b.x}
                y={padT + chartH - b.failH}
                width={barW}
                height={b.failH}
                fill="var(--color-error)"
                rx={2}
              />
            )}
            {/* Date label */}
            <text
              x={b.x + barW / 2}
              y={h - 6}
              textAnchor="middle"
              fontSize={9}
              fill="var(--color-text-tertiary)"
              transform={data.length > 14 ? `rotate(-35, ${b.x + barW / 2}, ${h - 6})` : undefined}
            >
              {b.date.slice(5)}
            </text>
          </g>
        ))}
      </svg>
    );
  }, [data, height]);

  if (data.length === 0) {
    return (
      <div style={{ height, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--color-text-tertiary)', fontSize: 13 }}>
        暂无数据
      </div>
    );
  }

  return (
    <div style={{ width: '100%' }}>
      <div style={{ display: 'flex', gap: 16, marginBottom: 8, justifyContent: 'flex-end' }}>
        <span style={{ fontSize: 11, color: 'var(--color-success)', display: 'flex', alignItems: 'center', gap: 4 }}>
          <span style={{ width: 8, height: 8, borderRadius: 2, background: 'var(--color-success)' }} />
          成功
        </span>
        <span style={{ fontSize: 11, color: 'var(--color-error)', display: 'flex', alignItems: 'center', gap: 4 }}>
          <span style={{ width: 8, height: 8, borderRadius: 2, background: 'var(--color-error)' }} />
          失败
        </span>
      </div>
      {svg}
    </div>
  );
}
