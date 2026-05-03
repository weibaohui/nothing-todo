import type { ChatMessage } from '../types';

const STATUS_MAP: Record<string, string> = {
  success: '成功',
  failed: '失败',
  running: '运行中',
};

export function conversationToMarkdown(
  messages: ChatMessage[],
  meta?: {
    title?: string;
    executor?: string;
    model?: string;
    startedAt?: string;
    status?: string;
  },
): string {
  const lines: string[] = [];

  // Header
  lines.push('# 执行记录');
  lines.push('');
  if (meta?.title) lines.push(`- **任务**: ${meta.title}`);
  if (meta?.executor) lines.push(`- **执行器**: ${meta.executor}`);
  if (meta?.model) lines.push(`- **模型**: ${meta.model}`);
  if (meta?.startedAt) lines.push(`- **开始时间**: ${meta.startedAt}`);
  if (meta?.status) lines.push(`- **状态**: ${STATUS_MAP[meta.status] || meta.status}`);
  lines.push('---');
  lines.push('');

  for (const msg of messages) {
    switch (msg.role) {
      case 'user':
        lines.push('## 👤 用户');
        lines.push('');
        lines.push(msg.content);
        lines.push('');
        lines.push('---');
        lines.push('');
        break;
      case 'assistant':
        lines.push('## 🤖 助手');
        lines.push('');
        lines.push(msg.content);
        lines.push('');
        lines.push('---');
        lines.push('');
        break;
      case 'thinking':
        lines.push('## 💡 思考过程');
        lines.push('');
        lines.push(msg.content);
        lines.push('');
        lines.push('---');
        lines.push('');
        break;
      case 'tool':
        lines.push(`## 🔧 工具调用: ${msg.toolName || '工具'}`);
        lines.push('');
        if (msg.toolInput) {
          lines.push('**输入参数:**');
          lines.push('');
          lines.push('```json');
          lines.push(msg.toolInput);
          lines.push('```');
          lines.push('');
        }
        if (msg.toolResult) {
          lines.push('**执行结果:**');
          lines.push('');
          lines.push('```');
          lines.push(truncate(msg.toolResult, 5000));
          lines.push('```');
          lines.push('');
        }
        lines.push('---');
        lines.push('');
        break;
      case 'result':
        lines.push('## ✅ 执行结果');
        lines.push('');
        lines.push(msg.content);
        lines.push('');
        lines.push('---');
        lines.push('');
        break;
      case 'system':
        lines.push('> ' + msg.content);
        lines.push('');
        break;
    }
  }

  lines.push(`*导出时间: ${new Date().toLocaleString()}*`);
  lines.push('');

  return lines.join('\n');
}

function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen) + '\n... (已截断)';
}
