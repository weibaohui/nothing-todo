/**
 * 剪贴板操作工具函数
 * 解决 HTTP 环境下 navigator.clipboard API 不可用的问题
 */

/**
 * 复制文本到剪贴板
 * 优先使用现代 Clipboard API，不支持时 fallback 到 execCommand
 * @param text 要复制的文本
 * @returns Promise<boolean> 是否复制成功
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  // 优先使用现代 Clipboard API（需要 HTTPS 或 localhost）
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch (error) {
      console.warn('Clipboard API 复制失败，尝试 fallback:', error);
      // 降级到 execCommand
      return fallbackCopyToClipboard(text);
    }
  }
  
  // Clipboard API 不可用，使用 fallback
  return fallbackCopyToClipboard(text);
}

/**
 * Fallback 复制方法：使用 textarea + execCommand
 * 适用于 HTTP 环境或旧浏览器
 * @param text 要复制的文本
 * @returns boolean 是否复制成功
 */
function fallbackCopyToClipboard(text: string): boolean {
  // 创建临时 textarea 元素
  const textarea = document.createElement('textarea');
  
  // 设置样式：不可见但可选中
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
  
  // 设置要复制的文本
  textarea.value = text;
  
  // 添加到 DOM
  document.body.appendChild(textarea);
  
  try {
    // 选中文本
    textarea.select();
    textarea.setSelectionRange(0, textarea.value.length);
    
    // 执行复制命令
    const successful = document.execCommand('copy');
    return successful;
  } catch (error) {
    console.error('execCommand 复制失败:', error);
    return false;
  } finally {
    // 清理：移除临时元素
    document.body.removeChild(textarea);
  }
}
