import { getExecutorOption } from '../types';

interface ExecutorBadgeProps {
  executor: string;
  className?: string;
  style?: React.CSSProperties;
}

export function ExecutorBadge({ executor, className, style }: ExecutorBadgeProps) {
  const opt = getExecutorOption(executor);
  return (
    <span
      className={className}
      style={{
        backgroundColor: `${opt.color}12`,
        color: opt.color,
        border: `1px solid ${opt.color}30`,
        display: 'inline-flex',
        alignItems: 'center',
        gap: 4,
        padding: '2px 8px',
        borderRadius: 4,
        fontSize: 12,
        fontWeight: 600,
        ...style,
      }}
    >
      {opt.icon} {opt.label}
    </span>
  );
}
