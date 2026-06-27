import { useState } from 'react';
import { Input, Button } from 'antd';
import { SendOutlined } from '@ant-design/icons';
import type { ExecutionRecord } from '@/types';

/**
 * 论坛式内联回复框 —— 替代原来的 Modal "继续对话"。
 * 输入框 + 回复按钮，紧凑布局。
 */
export function ReplyInput({
  record,
  onReply,
  loading,
}: {
  record: ExecutionRecord;
  onReply: (record: ExecutionRecord, message: string) => Promise<void>;
  loading?: boolean;
}) {
  const [message, setMessage] = useState('');

  const handleReply = async () => {
    const trimmed = message.trim();
    if (!trimmed || loading) return;
    await onReply(record, trimmed);
    setMessage('');
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleReply();
    }
  };

  return (
    <div style={{
      marginLeft: 24,
      marginTop: 4,
      marginBottom: 8,
      display: 'flex',
      gap: 8,
      alignItems: 'center',
    }}>
      <Input
        size="small"
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="输入回复内容..."
        disabled={loading}
        style={{
          flex: 1,
          borderRadius: 16,
          fontSize: 12,
        }}
      />
      <Button
        type="primary"
        size="small"
        icon={<SendOutlined />}
        onClick={handleReply}
        loading={loading}
        disabled={!message.trim()}
        style={{ borderRadius: 16 }}
      >
        回复
      </Button>
    </div>
  );
}
