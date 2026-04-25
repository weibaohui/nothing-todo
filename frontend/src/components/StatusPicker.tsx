import { useState } from 'react';
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
  const [open, setOpen] = useState(false);

  const current = statusConfig[value] || statusConfig.pending;

  const handleSelect = (status: string) => {
    onChange(status);
    setOpen(false);
  };

  const triggerNode = (
    <div
      className="status-picker-trigger"
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
      tabIndex={disabled ? -1 : 0}
      aria-label={`当前状态: ${current.label}`}
      onClick={(e) => e.stopPropagation()}
    />
  );

  if (disabled) {
    return triggerNode;
  }

  return (
    <Popover
      content={
        <div className="status-picker-popover">
          {Object.entries(statusConfig).map(([key, config]) => (
            <div
              key={key}
              className={`status-picker-item ${value === key ? 'active' : ''}`}
              onClick={() => handleSelect(key)}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  handleSelect(key);
                }
              }}
            >
              <span
                className="status-picker-circle"
                style={{ backgroundColor: config.color }}
              />
              <span className="status-picker-label">{config.label}</span>
              {value === key && (
                <CheckOutlined style={{ color: 'var(--color-primary)', fontWeight: 700, fontSize: 12 }} />
              )}
            </div>
          ))}
        </div>
      }
      trigger="click"
      open={open}
      onOpenChange={setOpen}
      placement="bottomLeft"
      getPopupContainer={() => document.body}
      zIndex={1050}
      destroyTooltipOnHide
    >
      {triggerNode}
    </Popover>
  );
}
