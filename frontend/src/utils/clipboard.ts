/**
 * Clipboard utility with fallback for non-secure contexts (HTTP)
 * 
 * The modern Clipboard API (navigator.clipboard.writeText) only works in secure contexts:
 * - HTTPS
 * - localhost
 * 
 * When accessing via HTTP (e.g., http://192.168.1.100:18088), the API is undefined.
 * This utility provides a fallback using document.execCommand('copy') with a textarea.
 */

/**
 * Copy text to clipboard with fallback for non-secure HTTP environments
 * @param text - The text to copy to clipboard
 * @returns Promise<boolean> - true if successful, false otherwise
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  // 优先使用现代 Clipboard API（HTTPS/localhost 环境）
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      // Clipboard API 失败，继续尝试 fallback
    }
  }

  // Fallback: 使用 textarea + execCommand（HTTP 环境）
  return fallbackCopyText(text);
}

/**
 * Fallback copy method using textarea and execCommand
 * 在 HTTP 环境中，Clipboard API 不可用，需要通过创建临时 textarea 元素并选中内容来复制
 * @param text - The text to copy
 * @returns boolean - true if successful, false otherwise
 */
function fallbackCopyText(text: string): boolean {
  // 创建临时 textarea 元素（不可见）
  const textarea = document.createElement('textarea');
  
  // 设置样式使其不可见但可交互（display:none 会导致 select() 失效）
  textarea.style.position = 'fixed';
  textarea.style.top = '0';
  textarea.style.left = '0';
  textarea.style.width = '2em';
  textarea.style.height = '2em';
  textarea.style.padding = '0';
  textarea.style.border = 'none';
  textarea.style.outline = 'none';
  textarea.style.boxShadow = 'none';
  textarea.style.background = 'transparent';
  textarea.style.opacity = '0';
  
  // 设置值并添加到 DOM
  textarea.value = text;
  document.body.appendChild(textarea);
  
  try {
    // 选中内容并执行复制命令
    textarea.select();
    textarea.setSelectionRange(0, text.length); // iOS 需要
    const successful = document.execCommand('copy');
    return successful;
  } catch (err) {
    return false;
  } finally {
    // 清理：移除临时元素
    document.body.removeChild(textarea);
  }
}
