/**
 * 将后端返回的 UTC ISO 8601 时间字符串解析为 Date 对象
 */
export function parseUtcDate(timeStr: string | null | undefined): Date | null {
  if (!timeStr) return null;
  return new Date(timeStr);
}

/**
 * 将 UTC 时间字符串格式化为本地时区的可读字符串
 */
export function formatLocalDateTime(timeStr: string | null | undefined): string {
  const date = parseUtcDate(timeStr);
  if (!date) return '';
  return date.toLocaleString();
}

/**
 * 将时间格式化为相对时间（多久之前）
 */
export function formatRelativeTime(timeStr: string | null | undefined): string {
  const date = parseUtcDate(timeStr);
  if (!date) return '';

  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 10) return '刚刚';
  if (diffSec < 60) return `${diffSec} 秒前`;
  if (diffMin < 60) return `${diffMin} 分钟前`;
  if (diffHour < 24) return `${diffHour} 小时前`;
  if (diffDay === 1) return '昨天';
  if (diffDay < 7) return `${diffDay} 天前`;

  return date.toLocaleDateString('zh-CN', {
    month: 'numeric',
    day: 'numeric',
  });
}

/**
 * 格式化时长（秒）为简写形式，最多4位
 * 90秒以内显示 XXs，超过90秒换算为 d/h/m
 */
export function formatDuration(seconds: number): string {
  // 90秒以内直接显示秒
  if (seconds < 90) return `${Math.floor(seconds)}s`;

  // 换算成更大的单位
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);

  if (d > 0) return `${d}d${h > 0 ? h + 'h' : ''}${m > 0 ? m + 'm' : ''}`.slice(0, 4);
  if (h > 0) return `${h}h${m > 0 ? m + 'm' : ''}`.slice(0, 4);
  return `${m}m`;
}

/**
 * 计算从指定时间到现在经过的秒数
 */
export function elapsedSeconds(startTimeStr: string | null | undefined): number {
  const date = parseUtcDate(startTimeStr);
  if (!date) return 0;
  const now = new Date();
  return Math.floor((now.getTime() - date.getTime()) / 1000);
}
