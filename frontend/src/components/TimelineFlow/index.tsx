import React, { useMemo } from 'react';
import { Tooltip } from 'antd';
import { motion, AnimatePresence } from 'framer-motion';

export interface TimelineRecord {
  id: string;
  role?: string;
  content?: string;
  timestamp?: number;
  event_type?: string;
  total_tokens?: number;
  [key: string]: unknown;
}

export interface TimelineFlowProps {
  records: TimelineRecord[];
  height?: number | string;
  colorMap?: Record<string, string>;
  labelMap?: Record<string, string>;
  disabled?: boolean;
  staggerDelay?: number;
  containerStyle?: React.CSSProperties;
  renderTooltip?: (record: TimelineRecord) => React.ReactNode;
}

const DEFAULT_COLOR_MAP: Record<string, string> = {
  user: 'var(--color-info)',
  assistant: 'var(--color-success)',
  system: 'var(--color-primary-bg)',
  tool: 'var(--color-warning)',
  tool_result: 'var(--color-primary)',
  text: 'var(--color-success)',
  thinking: 'var(--color-warning)',
  step_start: 'var(--color-primary-bg)',
  step_finish: 'var(--color-success-bg)',
  error: 'var(--color-error)',
  info: 'var(--color-primary)',
  stdout: 'var(--color-text-tertiary)',
  stderr: 'var(--color-text-tertiary)',
};

const DEFAULT_LABEL_MAP: Record<string, string> = {
  user: '用户',
  assistant: '助手',
  system: '系统',
  tool: '工具',
  tool_result: '结果',
  tool_call: '工具',
  tool_use: '工具',
  text: '输出',
  thinking: '思考',
  step_start: '开始',
  step_finish: '结束',
  error: '错误',
  info: '信息',
  stdout: '输出',
  stderr: '日志',
  result: '结果',
  tokens: '统计',
};

function formatTooltipContent(record: TimelineRecord, labelMap: Record<string, string>): React.ReactNode {
  const role = (record.role || '').toLowerCase();
  const label = labelMap[role] || role || '未知';
  return (
    <div>
      <div><strong>{label}</strong></div>
      {record.event_type && <div>类型: {record.event_type}</div>}
      {record.total_tokens != null && record.total_tokens > 0 && <div>Tokens: {record.total_tokens}</div>}
      {record.content && (
        <div style={{
          maxWidth: 300, overflow: 'hidden', textOverflow: 'ellipsis',
          whiteSpace: 'nowrap', marginTop: 4, opacity: 0.8, fontSize: 12,
        }}>
          {record.content}
        </div>
      )}
    </div>
  );
}

export const TimelineFlow: React.FC<TimelineFlowProps> = ({
  records,
  height = 24,
  colorMap = {},
  labelMap = {},
  disabled = false,
  staggerDelay = 0.08,
  containerStyle,
  renderTooltip,
}) => {
  const sortedRecords = useMemo(() => {
    return [...records].sort((a, b) => (a.timestamp || 0) - (b.timestamp || 0));
  }, [records]);

  const mergedColorMap = { ...DEFAULT_COLOR_MAP, ...colorMap };
  const mergedLabelMap = { ...DEFAULT_LABEL_MAP, ...labelMap };

  const displayRecords = sortedRecords;

  if (sortedRecords.length === 0) {
    return (
      <div style={{
        height, width: '100%',
        background: 'var(--color-border-light)',
        borderRadius: 4,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        color: 'var(--color-text-quaternary, var(--color-text-tertiary))',
        fontSize: 11,
        ...containerStyle,
      }}>
        暂无对话记录
      </div>
    );
  }

  const itemWidthPercent = 100 / displayRecords.length;

  return (
    <div style={{
      width: '100%', height,
      display: 'flex', flexDirection: 'row',
      borderRadius: 4, overflow: 'hidden',
      background: 'var(--color-border-light)',
      position: 'relative',
      ...containerStyle,
    }}>
      <AnimatePresence mode="popLayout">
        {displayRecords.map((record, index) => {
          const role = (record.role || '').toLowerCase();
          const color = mergedColorMap[role] || 'var(--color-text-quaternary, #bfbfbf)';

          const motionProps = disabled
            ? { initial: { x: 0, opacity: 1 }, animate: { x: 0, opacity: 1 }, exit: { opacity: 1, scale: 1 } }
            : {
                initial: { x: 300, opacity: 0 },
                animate: { x: 0, opacity: 1 },
                exit: { opacity: 0, scale: 0 },
              };

          const content = (
            <motion.div
              key={record.id}
              {...motionProps}
              transition={{
                type: 'spring', stiffness: 250, damping: 25,
                delay: disabled ? 0 : index * staggerDelay,
              }}
              style={{
                height: '100%',
                width: `${itemWidthPercent}%`,
                background: color,
                borderRight: index < displayRecords.length - 1 ? '1px solid var(--color-bg-elevated)' : 'none',
                cursor: 'pointer',
                minWidth: 3,
              }}
            />
          );

          return renderTooltip ? (
            <Tooltip key={record.id} title={renderTooltip(record)}>{content}</Tooltip>
          ) : (
            <Tooltip key={record.id} title={formatTooltipContent(record, mergedLabelMap)}>{content}</Tooltip>
          );
        })}
      </AnimatePresence>
    </div>
  );
};

export default TimelineFlow;
