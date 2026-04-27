import { AnimatedNumber } from './AnimatedNumber';

interface PieSegment {
  value: number;
  color: string;
  label: string;
}

interface PieChartProps {
  segments: PieSegment[];
  size?: number;
  centerText?: string;
  centerSubtext?: string;
}

export function PieChart({
  segments,
  size = 100,
  centerText,
  centerSubtext,
}: PieChartProps) {
  const total = segments.reduce((sum, s) => sum + s.value, 0);
  if (total === 0) return null;

  const cx = 50;
  const cy = 50;
  const r = 40;

  function polarToCartesian(
    cx: number,
    cy: number,
    r: number,
    angleDeg: number,
  ) {
    const angleRad = ((angleDeg - 90) * Math.PI) / 180;
    return { x: cx + r * Math.cos(angleRad), y: cy + r * Math.sin(angleRad) };
  }

  function describeArc(
    cx: number,
    cy: number,
    r: number,
    startAngle: number,
    endAngle: number,
  ) {
    const start = polarToCartesian(cx, cy, r, endAngle);
    const end = polarToCartesian(cx, cy, r, startAngle);
    const largeArcFlag = endAngle - startAngle <= 180 ? '0' : '1';
    return `M ${cx} ${cy} L ${start.x} ${start.y} A ${r} ${r} 0 ${largeArcFlag} 0 ${end.x} ${end.y} Z`;
  }

  let currentAngle = 0;
  const paths = segments
    .filter((s) => s.value > 0)
    .map((seg) => {
      const angle = (seg.value / total) * 360;
      const start = currentAngle;
      const end = currentAngle + angle;
      currentAngle = end;
      return {
        d: describeArc(cx, cy, r, start, end),
        color: seg.color,
      };
    });

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 100 100"
      style={{ flexShrink: 0 }}
    >
      <circle cx={cx} cy={cy} r={r} fill="#e2e8f0" />
      {paths.map((p, i) => (
        <path key={i} d={p.d} fill={p.color} />
      ))}
      <circle cx={cx} cy={cy} r={26} fill="#fff" />
      {centerText && (
        <text
          x={cx}
          y={cy + (centerSubtext ? -5 : 0)}
          textAnchor="middle"
          dominantBaseline="central"
          style={{
            fontSize: 13,
            fontWeight: 700,
            fill: '#0f172a',
            fontFamily: 'var(--font-sans)',
          }}
        >
          {centerText}
        </text>
      )}
      {centerSubtext && (
        <text
          x={cx}
          y={cy + 10}
          textAnchor="middle"
          dominantBaseline="central"
          style={{
            fontSize: 9,
            fontWeight: 500,
            fill: '#94a3b8',
            fontFamily: 'var(--font-sans)',
          }}
        >
          {centerSubtext}
        </text>
      )}
    </svg>
  );
}

export function PieChartLegend({
  segments,
  chineseFormat = false,
}: {
  segments: { value: number; color: string; label: string }[];
  chineseFormat?: boolean;
}) {
  return (
    <div
      style={{
        display: 'flex',
        flexWrap: 'wrap',
        gap: '8px 16px',
      }}
    >
      {segments.map((seg, i) => (
        <div
          key={i}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
          }}
        >
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              background: seg.color,
              flexShrink: 0,
            }}
          />
          <span style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>
            {seg.label}{' '}
            <strong style={{ color: 'var(--color-text)', fontWeight: 600 }}>
              <AnimatedNumber value={seg.value} duration={0.8} chineseFormat={chineseFormat} />
            </strong>
          </span>
        </div>
      ))}
    </div>
  );
}
