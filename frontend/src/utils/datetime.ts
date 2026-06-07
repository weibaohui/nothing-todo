/**
 * 解析后端返回的时间字符串为 Date 对象。
 * 数据库存的是本地时间（Asia/Shanghai），格式为 "YYYY-MM-DDTHH:MM:SSZ"（Z 是格式的一部分，非时区标识）。
 * 解析时需去掉末尾的 Z 当作本地时间字符串处理，避免 JS 将其误认为 UTC 时间。
 */
export function parseUtcDate(timeStr: string | null | undefined): Date | null {
  if (!timeStr) return null;
  // 去掉末尾的 Z，当作普通本地时间字符串解析
  const localStr = timeStr.endsWith('Z') ? timeStr.slice(0, -1) : timeStr;
  return new Date(localStr);
}

/**
 * 将时间格式化为本地可读字符串
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
  if (diffMs < 0) return '';

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
 * 格式化时长（秒）为简写形式，如 1h30m, 3m10s, 45s
 */
export function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h${m}m`;
  if (m > 0) return `${m}m`;
  return `${seconds}s`;
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
