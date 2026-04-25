import { useState, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
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
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const triggerRef = useRef<HTMLDivElement>(null);
  const current = statusConfig[value] || statusConfig.pending;

  useEffect(() => {
    if (isOpen && triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect();
      setPosition({
        top: rect.bottom + 4,
        left: rect.left
      });
    }
  }, [isOpen]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (triggerRef.current && !triggerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => {
        document.removeEventListener('mousedown', handleClickOutside);
      };
    }
  }, [isOpen]);

  const handleSelect = (status: string) => {
    if (status !== value) {
      onChange(status);
    }
    setIsOpen(false);
  };

  const triggerNode = (
    <div
      ref={triggerRef}
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
        position: 'relative',
      }}
      role="button"
      tabIndex={0}
      aria-label={`当前状态: ${current.label}`}
      onClick={(e) => {
        e.stopPropagation();
        if (!disabled) {
          setIsOpen(!isOpen);
        }
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          e.stopPropagation();
          if (!disabled) {
            setIsOpen(!isOpen);
          }
        }
      }}
    />
  );

  if (disabled) {
    return triggerNode;
  }

  return (
    <div style={{ display: 'inline-block', position: 'relative' }}>
      {triggerNode}
      {isOpen && createPortal(
        <div
          style={{
            position: 'fixed',
            top: position.top,
            left: position.left,
            minWidth: 140,
            backgroundColor: '#ffffff',
            borderRadius: 8,
            boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
            border: '1px solid #f0f0f0',
            zIndex: 2147483647, // 最大可能的 z-index
            padding: 4,
          }}
          onClick={(e) => e.stopPropagation()}
          onMouseDown={(e) => e.stopPropagation()}
        >
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
              }}
              onClick={(e) => {
                e.stopPropagation();
                e.preventDefault();
                handleSelect(key);
              }}
              onMouseDown={(e) => {
                e.stopPropagation();
                e.preventDefault();
              }}
              onMouseEnter={(e) => {
                if (value !== key) {
                  e.currentTarget.style.background = 'var(--color-bg)';
                }
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'transparent';
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
        </div>,
        document.body
      )}
    </div>
  );
}
