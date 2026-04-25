import { useState } from 'react';
import { Popover } from 'antd';

const statusConfig: Record<string, { color: string; label: string }> = {
  pending: { color: '#d9d9d9', label: '待执行' },
  running: { color: '#1890ff', label: '执行中' },
  completed: { color: '#52c41a', label: '已完成' },
  failed: { color: '#ff4d4f', label: '失败' },
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

  return (
    <Popover
      content={
        <div className="status-picker-popover">
          {Object.entries(statusConfig).map(([key, config]) => (
            <div
              key={key}
              className={`status-picker-item ${value === key ? 'active' : ''}`}
              onClick={() => handleSelect(key)}
            >
              <span
                className="status-picker-circle"
                style={{ backgroundColor: config.color }}
              />
              <span className="status-picker-label">{config.label}</span>
              {value === key && (
                <span className="status-picker-check">✓</span>
              )}
            </div>
          ))}
        </div>
      }
      trigger="click"
      open={open}
      onOpenChange={setOpen}
      placement="bottomLeft"
    >
      <span
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
          opacity: disabled ? 0.6 : 1,
          border: 'none',
        }}
        onClick={(e) => {
          if (!disabled) {
            e.stopPropagation();
            setOpen(true);
          }
        }}
      />
    </Popover>
  );
}
