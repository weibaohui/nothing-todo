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
