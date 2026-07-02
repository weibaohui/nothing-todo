import { useState } from 'react';
import { Button, Drawer, Spin, Typography, Space, message } from 'antd';
import { ThunderboltOutlined } from '@ant-design/icons';
import { useIsMobile } from '@/hooks/useIsMobile';
import { useActionExecution } from './useActionExecution';
import type { ActionButtonProps } from './types';

const { Text, Paragraph } = Typography;

/**
 * 可复用的一键 AI 执行组件。
 *
 * 交互流程：
 * 1. 点击按钮 → 打开 Drawer，展示参数预览（只读）
 * 2. 点击「执行」→ 调用 POST /api/actions/execute（查找或创建 todo → 执行）
 * 3. 通过 WebSocket 监听执行完成
 * 4. 完成后展示完整 markdown 结果
 * 5. 用户选择「应用」→ 调用 onApply 回调，或「拒绝」→ 关闭面板
 *
 * 后端逻辑：
 * - 根据 actionType + actionKey 查找 todo
 * - 如果不存在，自动创建 todo（prompt 来自请求）
 * - 执行该 todo，返回结果
 *
 * Prompt 模板语法：
 * - {{key}} → params 中 key 对应的值
 *
 * 示例：
 * prompt="优化标题：{{title}}，参考 Prompt：{{prompt}}"
 * params={{ title: "fix bug", prompt: "帮我修复登录超时" }}
 * → 执行消息="优化标题：fix bug，参考 Prompt：帮我修复登录超时"
 */
export function ActionButton({
  actionType,
  actionKey,
  prompt,
  params,
  onApply,
  workspaceId,
  children,
  buttonType = 'default',
  icon,
  disabled = false,
  panelTitle = '智能执行',
  panelDescription = '将使用 AI 处理以下内容',
  executor,
}: ActionButtonProps) {
  const [open, setOpen] = useState(false);
  const isMobile = useIsMobile();
  const { status, result, error, execute, retry, reset } = useActionExecution(
    actionType,
    actionKey,
    prompt,
    params,
    workspaceId,
    executor,
  );

  const handleOpen = () => {
    reset();
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
  };

  const handleApply = async () => {
    if (!result) return;
    try {
      await onApply(result);
      message.success('已应用');
      handleClose();
    } catch (err: any) {
      message.error(err?.message || '应用失败');
    }
  };

  // 从 params 中提取要展示的预览内容
  const previewContent = Object.entries(params)
    .map(([key, value]) => `${key}: ${value}`)
    .join('\n');

  const renderContent = () => {
    if (status === 'idle') {
      return (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Text type="secondary">{panelDescription}</Text>
          <div
            style={{
              padding: 12,
              background: '#f5f5f5',
              borderRadius: 6,
              maxHeight: 200,
              overflow: 'auto',
            }}
          >
            <Text ellipsis>
              {previewContent || '(空)'}
            </Text>
          </div>
        </Space>
      );
    }

    if (status === 'executing') {
      return (
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <Spin size="large" />
          <div style={{ marginTop: 16 }}>
            <Text type="secondary">AI 正在处理中...</Text>
          </div>
        </div>
      );
    }

    if (status === 'failed') {
      return (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Text type="danger">{error || '执行失败'}</Text>
        </Space>
      );
    }

    // completed
    return (
      <Space direction="vertical" size="middle" style={{ width: '100%' }}>
        <Text type="secondary">AI 生成结果：</Text>
        <div
          style={{
            padding: 12,
            background: '#f6ffed',
            border: '1px solid #b7eb8f',
            borderRadius: 6,
            maxHeight: 400,
            overflow: 'auto',
          }}
        >
          <Paragraph
            style={{ whiteSpace: 'pre-wrap', margin: 0 }}
            ellipsis={{ expandable: true, symbol: '展开' }}
          >
            {result}
          </Paragraph>
        </div>
      </Space>
    );
  };

  const renderFooter = () => {
    if (status === 'idle') {
      return (
        <Space>
          <Button onClick={handleClose}>取消</Button>
          <Button type="primary" onClick={execute}>
            执行
          </Button>
        </Space>
      );
    }

    if (status === 'executing') {
      return null;
    }

    if (status === 'failed') {
      return (
        <Space>
          <Button onClick={handleClose}>关闭</Button>
          <Button type="primary" onClick={retry}>
            重试
          </Button>
        </Space>
      );
    }

    // completed
    return (
      <Space>
        <Button onClick={handleClose}>拒绝</Button>
        <Button type="primary" onClick={handleApply}>
          应用
        </Button>
      </Space>
    );
  };

  return (
    <>
      <Button
        type={buttonType}
        icon={icon || <ThunderboltOutlined />}
        onClick={handleOpen}
        disabled={disabled}
      >
        {children || '智能执行'}
      </Button>

      <Drawer
        title={panelTitle}
        open={open}
        onClose={handleClose}
        placement={isMobile ? 'bottom' : 'right'}
        width={isMobile ? '100%' : 480}
        height={isMobile ? '80vh' : undefined}
        footer={renderFooter()}
        destroyOnClose
      >
        {renderContent()}
      </Drawer>
    </>
  );
}
