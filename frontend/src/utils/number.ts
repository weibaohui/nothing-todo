/**
 * 将数字格式化为中文阅读习惯的字符串
 * 例如: 1234 -> "1,234", 12345 -> "1.23万", 123456789 -> "1.23亿"
 */
export function formatChineseNumber(num: number): string {
  if (num === 0) return '0';

  const absNum = Math.abs(num);

  if (absNum < 10000) {
    return num.toLocaleString('zh-CN');
  }

  if (absNum < 100000000) {
    const wan = num / 10000;
    return `${wan.toFixed(2)}万`;
  }

  const yi = num / 100000000;
  return `${yi.toFixed(2)}亿`;
}
