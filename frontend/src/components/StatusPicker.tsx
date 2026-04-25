import { Popover } from 'antd';
import { CheckOutlined } from '@ant-design/icons';

const statusConfig: Record<string, { color: string; label: string; bg: string }> = {
  pending: { color: '#94a3b8', label: '待执行', bg: '#f1f5f9' },
  running: { color: '#3b82f6', label: '执行中', bg: '#eff6ff' },
  completed: { color: '#22c55e', label: '已完成', bg: '#f0fdf4' },
  failed: { color: '#ef4444', label: '失败', bg: '#fef2f2' },
};

interface StatusPickerProps {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}

export function StatusPicker({ value, onChange, disabled }: StatusPickerProps) {
  const current = statusConfig[value] || statusConfig.pending;

  const handleSelect = (status: string) => {
    if (status !== value) {
      onChange(status);
    }
  };

  const triggerNode = (
    <div
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: 28,
        height: 28,
        borderRadius: '50%',
        backgroundColor: current.color,
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.5 : 1,
        border: 'none',
        flexShrink: 0,
        transition: 'all 0.2s ease',
        boxShadow: `0 2px 6px ${current.color}40`,
      }}
      role="button"
      tabIndex={0}
      aria-label={`当前状态: ${current.label}`}
    />
  );

  if (disabled) {
    return triggerNode;
  }

  return (
    <Popover
      content={
        <div style={{ padding: 4, minWidth: 140 }}>
          {Object.entries(statusConfig).map(([key, config]) => (
            <div
              key={key}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 12,
                padding: '8px 12px',
                borderRadius: 6,
                cursor: 'pointer',
                transition: 'background 150ms ease',
                background: value === key ? 'var(--color-primary-bg)' : '#ffffff',
              }}
              onClick={() => handleSelect(key)}
              onMouseEnter={(e) => {
                if (value !== key) {
                  e.currentTarget.style.background = 'var(--color-bg)';
                }
              }}
              onMouseLeave={(e) => {
                if (value !== key) {
                  e.currentTarget.style.background = '#ffffff';
                }
              }}
            >
              <span
                style={{
                  width: 14,
                  height: 14,
                  borderRadius: '50%',
                  backgroundColor: config.color,
                  flexShrink: 0,
                }}
              />
              <span style={{ fontSize: 14, color: 'var(--color-text)', fontWeight: 500 }}>
                {config.label}
              </span>
              {value === key && <CheckOutlined style={{ color: 'var(--color-primary)', fontWeight: 700, fontSize: 12 }} />}
            </div>
          ))}
        </div>
      }
      trigger="click"
      placement="bottomLeft"
    >
      {triggerNode}
    </Popover>
  );
}
