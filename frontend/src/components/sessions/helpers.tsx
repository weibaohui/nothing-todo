import { Tag } from 'antd';
import { formatTokens, formatRelativeTimeFromNow } from '@/utils/format';

export const sourceConfig: Record<string, { label: string; color: string }> = {
  'claudecode': { label: 'Claude Code', color: '#d97706' },
  'codex': { label: 'Codex', color: '#10a37f' },
  'hermes': { label: 'Hermes', color: '#8b5cf6' },
  'kimi': { label: 'Kimi', color: '#3b82f6' },
  'atomcode': { label: 'AtomCode', color: '#ef4444' },
  'codebuddy': { label: 'CodeBuddy', color: '#f59e0b' },
  'opencode': { label: 'OpenCode', color: '#22c55e' },
  'mobilecoder': { label: 'MobileCoder', color: '#6366f1' },
  // Issue #673: zhanlu 与 opencode 输出格式一致，使用相近色便于视觉区分
  'zhanlu': { label: 'Zhanlu', color: '#0f766e' },
};

export function sourceTag(source: string) {
  const cfg = sourceConfig[source] || { label: source, color: '#6b7280' };
  return (
    <Tag color={cfg.color} style={{ fontSize: 11, lineHeight: '18px', padding: '0 6px' }}>
      {cfg.label}
    </Tag>
  );
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export { formatTokens, formatRelativeTimeFromNow as formatTime };

export function shortId(id: string): string {
  return id.length > 12 ? `${id.slice(0, 8)}...${id.slice(-4)}` : id;
}
