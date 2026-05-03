import yaml from 'js-yaml';
import type { ChatMessage } from '../types';

const STATUS_MAP: Record<string, string> = {
  success: '成功',
  failed: '失败',
  running: '运行中',
};

export function conversationToYaml(
  messages: ChatMessage[],
  meta?: {
    title?: string;
    executor?: string;
    model?: string;
    startedAt?: string;
    status?: string;
  },
): string {
  const header: Record<string, string> = {};
  if (meta?.title) header['任务'] = meta.title;
  if (meta?.executor) header['执行器'] = meta.executor;
  if (meta?.model) header['模型'] = meta.model;
  if (meta?.startedAt) header['开始时间'] = meta.startedAt;
  if (meta?.status) header['状态'] = STATUS_MAP[meta.status] || meta.status;

  const items = messages.map(msg => {
    const item: Record<string, unknown> = { role: msg.role };
    if (msg.timestamp) item['timestamp'] = msg.timestamp;
    switch (msg.role) {
      case 'user':
      case 'assistant':
      case 'thinking':
      case 'result':
        item['content'] = msg.content;
        break;
      case 'tool':
        item['name'] = msg.toolName || '工具';
        if (msg.toolInput) item['input'] = msg.toolInput;
        if (msg.toolResult) item['result'] = truncate(msg.toolResult, 5000);
        break;
      case 'system':
        item['content'] = msg.content;
        break;
    }
    return item;
  });

  const doc = {
    ...header,
    '导出时间': new Date().toLocaleString(),
    messages: items,
  };

  return yaml.dump(doc, { lineWidth: -1, forceQuotes: true, quotingType: "'" });
}

function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen) + '\n... (已截断)';
}
