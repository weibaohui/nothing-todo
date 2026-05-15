import { useMemo, useState, useEffect } from 'react';

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
        <path d={successPath} fill="none" stroke="var(--color-success)" strokeWidth={2} strokeLinejoin="round" />
        <path d={failPath} fill="none" stroke="var(--color-error)" strokeWidth={2} strokeLinejoin="round" />
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

interface DailyExecution {
  date: string;
  success: number;
  failed: number;
}

interface ContributionHeatmapProps {
  data: DailyExecution[];
}

export function ContributionHeatmap({ data }: ContributionHeatmapProps) {
  // 检测是否为暗色主题
  const [isDark, setIsDark] = useState(false);
  useEffect(() => {
    const checkTheme = () => {
      const bgColor = getComputedStyle(document.body).getPropertyValue('--ant-base-background-color').trim();
      // 如果背景色以 #0 或 rgb(0 开头，则是暗色主题
      setIsDark(bgColor.startsWith('#0') || bgColor.startsWith('rgb(0'));
    };
    checkTheme();
    // 监听主题变化
    const observer = new MutationObserver(checkTheme);
    observer.observe(document.body, { attributes: true, attributeFilter: ['style', 'class'] });
    return () => observer.disconnect();
  }, []);

  const { weeks } = useMemo(() => {
    if (data.length === 0) {
      return { weeks: [] };
    }

    const dateMap = new Map<string, number>();
    data.forEach((d) => {
      dateMap.set(d.date, d.success + d.failed);
    });

    // 2026年全年：1月1日到12月31日
    const startDate = new Date(2026, 0, 1); // 2026-01-01
    const endDate = new Date(2026, 11, 31); // 2026-12-31
    // 从周日开始对齐
    const dayOfWeek = startDate.getDay();
    startDate.setDate(startDate.getDate() - dayOfWeek);

    const weeksArr: { date: Date; count: number; level: number }[][] = [];
    let currentDate = new Date(startDate);
    let currentWeek: { date: Date; count: number; level: number }[] = [];

    while (currentDate <= endDate) {
      const year = currentDate.getFullYear();
      const month = currentDate.getMonth();
      const dayOfMonth = currentDate.getDate();
      const dateStr = `${year}-${String(month + 1).padStart(2, '0')}-${String(dayOfMonth).padStart(2, '0')}`;
      const count = dateMap.get(dateStr) || 0;

      currentWeek.push({ date: new Date(currentDate), count, level: 0 });

      if (currentDate.getDay() === 6) {
        weeksArr.push(currentWeek);
        currentWeek = [];
      }

      currentDate.setDate(currentDate.getDate() + 1);
    }

    if (currentWeek.length > 0) weeksArr.push(currentWeek);

    let max = 0;
    weeksArr.forEach((week) => week.forEach((day) => { max = Math.max(max, day.count); }));

    weeksArr.forEach((week) => {
      week.forEach((day) => {
        if (max === 0 || day.count === 0) {
          day.level = 0;
        } else if (day.count <= max * 0.25) {
          day.level = 1;
        } else if (day.count <= max * 0.5) {
          day.level = 2;
        } else if (day.count <= max * 0.75) {
          day.level = 3;
        } else {
          day.level = 4;
        }
      });
    });

    return { weeks: weeksArr };
  }, [data]);

  if (data.length === 0) {
    return <div style={{ height: 120, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--color-text-tertiary)', fontSize: 13 }}>暂无数据</div>;
  }

  // 动态计算格子大小，使热力图铺满容器，保持正方形
  const cellGap = 1;
  const weeksCount = weeks.length; // 53-54 周
  const daysCount = 7;

  // 用 viewBox 保持 53:7 比例
  const vbWidth = weeksCount * 10;
  const vbHeight = daysCount * 10;
  const cellSize = 9;

  // 计算高度百分比以保持正方形: 7/53 ≈ 13.2%
  const heightPercent = ((daysCount / weeksCount) * 100).toFixed(1);

  // 暗色主题用绿色系，亮色主题用蓝色系
  const levelColors = isDark
    ? ['var(--color-fill-quaternary)', '#9be9a8', '#40c463', '#30a14e', '#216e39']
    : ['var(--color-fill-quaternary)', '#dbeafe', '#93c5fd', '#3b82f6', '#1d4ed8'];

  return (
    <div style={{ width: '100%', paddingBottom: 8 }}>
      <div style={{ width: '100%', paddingBottom: `${heightPercent}%`, position: 'relative' }}>
        <svg width="100%" height="100%" viewBox={`0 0 ${vbWidth} ${vbHeight}`} preserveAspectRatio="xMidYMid meet" style={{ position: 'absolute', top: 0, left: 0 }}>
        {weeks.map((week, weekIndex) =>
          week.map((day, dayIndex) => (
            <rect
              key={`${weekIndex}-${dayIndex}`}
              x={weekIndex * (cellSize + cellGap)}
              y={dayIndex * (cellSize + cellGap)}
              width={cellSize}
              height={cellSize}
              rx={Math.max(1, cellSize / 4)}
              fill={levelColors[day.level]}
              style={{ cursor: 'pointer', transition: 'opacity 0.15s' }}
              onMouseEnter={(e) => {
                const tooltip = document.getElementById('heatmap-tooltip');
                if (tooltip) {
                  const dateStr = day.date.toLocaleDateString('zh-CN', { year: 'numeric', month: 'long', day: 'numeric' });
                  tooltip.textContent = day.count > 0 ? `${day.count} 次执行 · ${dateStr}` : `无执行 · ${dateStr}`;
                  tooltip.style.display = 'block';
                  tooltip.style.left = `${e.clientX + 10}px`;
                  tooltip.style.top = `${e.clientY - 30}px`;
                }
              }}
              onMouseLeave={() => { const tooltip = document.getElementById('heatmap-tooltip'); if (tooltip) tooltip.style.display = 'none'; }}
            />
          ))
        )}
      </svg>
      </div>
      <div id="heatmap-tooltip" style={{ display: 'none', position: 'fixed', background: 'var(--color-fill-elevated)', border: '1px solid var(--color-border)', borderRadius: 6, padding: '6px 10px', fontSize: 12, color: 'var(--color-text)', boxShadow: '0 2px 8px rgba(0,0,0,0.15)', zIndex: 1000, pointerEvents: 'none' }} />
      <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginTop: 8, justifyContent: 'flex-end' }}>
        <span style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginRight: 4 }}>少</span>
        {levelColors.map((color, i) => <div key={i} style={{ width: 10, height: 10, borderRadius: 2, background: color }} />)}
        <span style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginLeft: 4 }}>多</span>
      </div>
    </div>
  );
}
