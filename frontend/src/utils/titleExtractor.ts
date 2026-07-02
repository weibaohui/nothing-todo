/**
 * 从 AI 生成的结果中提取标题。
 *
 * 策略：
 * 1. 去除首尾空白
 * 2. 如果是纯文本（无 markdown），直接返回
 * 3. 如果包含 markdown，尝试提取第一个标题或加粗文本
 * 4. 如果都失败，返回清理后的文本
 *
 * 示例输入/输出：
 * - "登录超时问题修复" → "登录超时问题修复"
 * - "**登录超时问题修复**" → "登录超时问题修复"
 * - "# 登录超时问题修复" → "登录超时问题修复"
 * - "根据分析，建议改为：**登录超时问题修复**" → "登录超时问题修复"
 */
export function extractTitle(result: string): string {
  if (!result) return '';

  // 1. 去除首尾空白
  let text = result.trim();

  // 2. 尝试提取 markdown 标题 (# Title)
  const headingMatch = text.match(/^#{1,6}\s+(.+)$/m);
  if (headingMatch) {
    return cleanMarkdown(headingMatch[1].trim());
  }

  // 3. 尝试提取加粗文本 (**text** 或 __text__)
  const boldMatch = text.match(/\*\*(.+?)\*\*|__(.+?)__/);
  if (boldMatch) {
    return cleanMarkdown((boldMatch[1] || boldMatch[2]).trim());
  }

  // 4. 尝试提取引号内的文本 ("text" 或 「text」)
  const quoteMatch = text.match(/["「](.+?)["」]/);
  if (quoteMatch) {
    return cleanMarkdown(quoteMatch[1].trim());
  }

  // 5. 如果只有一行，直接返回
  const lines = text.split('\n').filter(l => l.trim());
  if (lines.length === 1) {
    return cleanMarkdown(lines[0].trim());
  }

  // 6. 多行文本：返回第一行（通常是标题）
  return cleanMarkdown(lines[0].trim());
}

/**
 * 清理 markdown 格式标记。
 */
function cleanMarkdown(text: string): string {
  return text
    .replace(/\*\*(.+?)\*\*/g, '$1')   // **bold**
    .replace(/__(.+?)__/g, '$1')         // __bold__
    .replace(/\*(.+?)\*/g, '$1')         // *italic*
    .replace(/_(.+?)_/g, '$1')           // _italic_
    .replace(/`(.+?)`/g, '$1')           // `code`
    .replace(/~~(.+?)~~/g, '$1')         // ~~strikethrough~~
    .trim();
}
