import { useMemo, useEffect, useRef } from 'react';
import CalHeatmapLib from 'cal-heatmap';
import Tooltip from 'cal-heatmap/plugins/Tooltip';
import Legend from 'cal-heatmap/plugins/Legend';
import dayjs from 'dayjs';

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

    const maxVal = Math.max(...data.map((d) => Math.max(d.success, d.failed)), 1);

    const points = data.map((d, i) => {
      const x = padL + (i / Math.max(data.length - 1, 1)) * chartW;
      const succY = padT + chartH - (d.success / maxVal) * chartH;
      const failY = padT + chartH - (d.failed / maxVal) * chartH;
      return { x, succY, failY, date: d.date };
    });

    const yTicks = [0, maxVal * 0.5, maxVal];

    const successPath = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.succY}`).join(' ');
    const failPath = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.failY}`).join(' ');

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

        {/* Success line */}
        <path d={successPath} fill="none" stroke="var(--color-success)" strokeWidth={2} strokeLinejoin="round" />
        {/* Fail line */}
        <path d={failPath} fill="none" stroke="var(--color-error)" strokeWidth={2} strokeLinejoin="round" />

        {/* Dots and date labels */}
        {points.map((p, i) => (
          <g key={i}>
            <circle cx={p.x} cy={p.succY} r={3} fill="var(--color-success)" />
            <circle cx={p.x} cy={p.failY} r={3} fill="var(--color-error)" />
            <text
              x={p.x}
              y={h - 6}
              textAnchor="middle"
              fontSize={9}
              fill="var(--color-text-tertiary)"
              transform={data.length > 14 ? `rotate(-35, ${p.x}, ${h - 6})` : undefined}
            >
              {p.date.slice(5)}
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

interface ContributionHeatmapProps {
  data: { date: string; success: number; failed: number }[];
}

export function ContributionHeatmap({ data }: ContributionHeatmapProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const legendRef = useRef<HTMLDivElement>(null);
  const calRef = useRef<ReturnType<typeof CalHeatmapLib> | null>(null);

  useEffect(() => {
    if (!containerRef.current || data.length === 0) return;

    // Destroy previous instance
    if (calRef.current) {
      calRef.current.destroy();
    }

    // Transform data: {date: "2024-01-15", success: 3, failed: 1} -> {timestamp: count}
    const heatmapData: Record<number, number> = {};
    data.forEach((d) => {
      const timestamp = new Date(d.date).getTime() / 1000;
      heatmapData[timestamp] = d.success + d.failed;
    });

    const cal = new CalHeatmapLib();
    calRef.current = cal;

    const startDate = data.length > 0 ? new Date(data[0].date) : new Date();

    cal.paint(
      {
        data: {
          source: heatmapData,
          type: 'json',
          x: 't',
          y: 'v',
        },
        date: { start: startDate },
        range: 4,
        scale: {
          color: {
            type: 'linear',
            scheme: 'PuBuGn',
            domain: [0, Math.max(...data.map((d) => d.success + d.failed), 1)],
          },
        },
        domain: {
          type: 'month',
          label: { text: null },
        },
        subDomain: { type: 'day', radius: 2, label: null },
        itemSelector: '#heatmap-container',
      },
      [
        [
          Tooltip,
          {
            text: function (date: Date, value: number) {
              return (
                (value ? value + ' 次执行' : '无执行') +
                ' on ' +
                dayjs(date).format('YYYY-MM-DD')
              );
            },
          },
        ],
        [
          Legend,
          {
            tickSize: 0,
            width: 120,
            itemSelector: legendRef.current || '#heatmap-legend',
            label: '执行次数',
          },
        ],
      ]
    );

    return () => {
      if (calRef.current) {
        calRef.current.destroy();
        calRef.current = null;
      }
    };
  }, [data]);

  if (data.length === 0) {
    return (
      <div style={{ height: 120, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--color-text-tertiary)', fontSize: 13 }}>
        暂无数据
      </div>
    );
  }

  return (
    <div>
      <div id="heatmap-container" ref={containerRef} style={{ overflowX: 'auto' }} />
      <div id="heatmap-legend" ref={legendRef} style={{ marginTop: 8 }} />
    </div>
  );
}
