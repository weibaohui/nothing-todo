import { useState, useEffect } from 'react';
import { ChatView } from '@/components/ChatView';
import { CommandPanel } from '@/components/CommandPanel';
import { LogViewHeader } from './LogViewHeader';
import { formatLogTime } from './helpers';
import * as db from '@/utils/database';
import type { LogEntry, ExecutionRecord } from '@/types';

/**
 * 续轮记录懒加载日志视图。
 *
 * 与 ContinuationLogView 互斥使用：当前 chain group 内若已有 logs 用前者，
 * 否则用本组件按需拉取一次。三种 viewMode 与 NarrowLogView 对齐：
 * - 'log'：原始日志列表
 * - 'chat'：对话视图
 * - 'command'：命令视图（CommandPanel）
 */
export function ContinuationLogsLoader({ record, viewMode, onRefresh, onViewModeChange }: {
  record: ExecutionRecord;
  viewMode: 'log' | 'chat' | 'command';
  onRefresh: (id: number) => Promise<void>;
  onViewModeChange: (mode: 'log' | 'chat' | 'command') => void;
}) {
  const [logs, setLogs] = useState<LogEntry[] | null>(null);
  // 切到「对话/命令」视图时直接展开，避免用户多次点击。
  const [isExpanded, setIsExpanded] = useState(viewMode === 'chat' || viewMode === 'command');
  useEffect(() => {
    db.getExecutionLogs(record.id, 1, 200)
      .then(r => setLogs(r.logs))
      .catch(() => setLogs([]));
  }, [record.id]);
  if (logs === null) return null;
  if (logs.length === 0) return null;
  // 抽 titleMap 替代三元嵌套：新增视图模式只需改这张表。
  const titleMap = { log: `日志 (${logs.length})`, chat: `对话 (${logs.length})`, command: `命令 (${logs.length})` } as const;
  const title = titleMap[viewMode];
  return (
    <details style={{ marginTop: 6 }} open={isExpanded} onToggle={(e) => setIsExpanded((e.target as HTMLDetailsElement).open)}>
      <summary style={{ cursor: 'pointer', color: 'var(--color-primary)', fontSize: 10, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 }}>
        <span>{title}</span>
        <LogViewHeader
          title=""
          viewMode={viewMode}
          onViewModeChange={onViewModeChange}
          onRefresh={() => onRefresh(record.id)}
          fontSize={10}
        />
      </summary>
      {viewMode === 'chat' ? (
        <div style={{ maxHeight: 300, overflow: 'auto' }}>
          <ChatView logs={logs as LogEntry[]} isRunning={false} />
        </div>
      ) : viewMode === 'command' ? (
        <div style={{ maxHeight: 300, overflow: 'auto' }}>
          <CommandPanel logs={logs} executor={record.executor} />
        </div>
      ) : (
        <div style={{
          background: 'var(--log-bg)', color: 'var(--log-text)', padding: 6, borderRadius: 6,
          fontFamily: 'var(--font-mono)', fontSize: 10, maxHeight: 200, overflow: 'auto',
        }}>
          {logs.map((log, i) => (
            <div key={i} style={{ marginBottom: 3, display: 'flex', gap: 6 }}>
              <span style={{ color: 'var(--log-text-muted)', flexShrink: 0 }}>{formatLogTime(log.timestamp || '')}</span>
              <span>{log.content ?? ''}</span>
            </div>
          ))}
        </div>
      )}
    </details>
  );
}
