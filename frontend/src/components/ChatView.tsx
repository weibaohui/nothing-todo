import { useState, useRef, useEffect } from 'react';
import { RobotOutlined, UserOutlined, ToolOutlined, BulbOutlined, CheckCircleOutlined, InfoCircleOutlined, LoadingOutlined } from '@ant-design/icons';
import XMarkdown from '@ant-design/x-markdown';
import type { LogEntry } from '../types';

interface ChatViewProps {
  logs: LogEntry[];
  isRunning?: boolean;
}

interface ChatMessage {
  role: 'user' | 'assistant' | 'system' | 'tool' | 'thinking' | 'result';
  content: string;
  timestamp?: string;
  toolName?: string;
  toolInput?: string;
  toolResult?: string;
  isCollapsed?: boolean;
}

function parseLogsToMessages(logs: LogEntry[]): ChatMessage[] {
  const messages: ChatMessage[] = [];
  let currentThinking = '';
  let currentToolName = '';
  let currentToolInput = '';
  let isCollectingTool = false;

  for (const log of logs) {
    switch (log.type) {
      case 'user':
        messages.push({ role: 'user', content: log.content, timestamp: log.timestamp });
        break;
      case 'assistant':
        if (currentThinking) {
          messages.push({ role: 'thinking', content: currentThinking, timestamp: log.timestamp, isCollapsed: true });
          currentThinking = '';
        }
        if (isCollectingTool && currentToolName) {
          messages.push({ role: 'tool', content: '', timestamp: log.timestamp, toolName: currentToolName, toolInput: currentToolInput, isCollapsed: true });
          currentToolName = '';
          currentToolInput = '';
          isCollectingTool = false;
        }
        messages.push({ role: 'assistant', content: log.content, timestamp: log.timestamp });
        break;
      case 'thinking':
        currentThinking += log.content + '\n';
        break;
      case 'tool':
      case 'tool_use':
      case 'tool_call':
        if (isCollectingTool && currentToolName) {
          messages.push({ role: 'tool', content: '', timestamp: log.timestamp, toolName: currentToolName, toolInput: currentToolInput, isCollapsed: true });
          currentToolName = '';
          currentToolInput = '';
          isCollectingTool = false;
        }
        if (currentThinking) {
          messages.push({ role: 'thinking', content: currentThinking, timestamp: log.timestamp, isCollapsed: true });
          currentThinking = '';
        }
        try {
          const toolData = JSON.parse(log.content);
          currentToolName = toolData.name || toolData.tool || '';
          currentToolInput = toolData.input ? JSON.stringify(toolData.input, null, 2) : log.content;
          isCollectingTool = true;
        } catch {
          currentToolName = log.content;
          currentToolInput = '';
          isCollectingTool = true;
        }
        break;
      case 'tool_result':
        if (isCollectingTool && currentToolName) {
          messages.push({ role: 'tool', content: '', timestamp: log.timestamp, toolName: currentToolName, toolInput: currentToolInput, toolResult: log.content, isCollapsed: true });
          currentToolName = '';
          currentToolInput = '';
          isCollectingTool = false;
        } else {
          messages.push({ role: 'tool', content: log.content, timestamp: log.timestamp, isCollapsed: true });
        }
        break;
      case 'result':
        if (currentThinking) {
          messages.push({ role: 'thinking', content: currentThinking, timestamp: log.timestamp, isCollapsed: true });
          currentThinking = '';
        }
        if (isCollectingTool && currentToolName) {
          messages.push({ role: 'tool', content: '', timestamp: log.timestamp, toolName: currentToolName, toolInput: currentToolInput, isCollapsed: true });
          currentToolName = '';
          currentToolInput = '';
          isCollectingTool = false;
        }
        messages.push({ role: 'result', content: log.content, timestamp: log.timestamp });
        break;
      case 'info':
      case 'system':
      case 'stdout':
      case 'stderr':
      case 'error':
      case 'text':
      case 'step_start':
      case 'step_finish':
      case 'tokens':
        messages.push({ role: 'system', content: log.content, timestamp: log.timestamp });
        break;
    }
  }

  // Flush remaining
  if (currentThinking) {
    messages.push({ role: 'thinking', content: currentThinking });
  }
  if (isCollectingTool && currentToolName) {
    messages.push({ role: 'tool', content: '', toolName: currentToolName, toolInput: currentToolInput });
  }

  return messages;
}

function formatTime(iso?: string): string {
  if (!iso) return '';
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
  } catch {
    return '';
  }
}

function ThinkingBlock({ content, timestamp }: { content: string; timestamp?: string }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className="chat-thinking-block">
      <button
        type="button"
        className="chat-thinking-header"
        aria-expanded={expanded}
        onClick={() => setExpanded(!expanded)}
      >
        <BulbOutlined style={{ color: '#f59e0b' }} />
        <span>思考过程</span>
        <span className="chat-thinking-toggle">{expanded ? '收起' : '展开'}</span>
        {timestamp && <span className="chat-time">{formatTime(timestamp)}</span>}
      </button>
      {expanded && (
        <div className="chat-thinking-content">
          <XMarkdown content={content} />
        </div>
      )}
    </div>
  );
}

function ToolBlock({ toolName, toolInput, toolResult, timestamp }: { toolName?: string; toolInput?: string; toolResult?: string; timestamp?: string }) {
  const [expanded, setExpanded] = useState(false);

  // 生成参数预览（截取前50个字符）
  const getInputPreview = () => {
    if (!toolInput) return '';
    try {
      const parsed = JSON.parse(toolInput);
      const keys = Object.keys(parsed);
      if (keys.length === 0) return '{}';
      const preview = keys.map(k => `${k}: ${typeof parsed[k] === 'string' ? `"${parsed[k].substring(0, 20)}${parsed[k].length > 20 ? '...' : ''}"` : parsed[k]}`).join(', ');
      return preview.length > 60 ? preview.substring(0, 60) + '...' : preview;
    } catch {
      return toolInput.length > 50 ? toolInput.substring(0, 50) + '...' : toolInput;
    }
  };

  return (
    <div className="chat-tool-block">
      <button
        type="button"
        className="chat-tool-header"
        aria-expanded={expanded}
        onClick={() => setExpanded(!expanded)}
      >
        <ToolOutlined style={{ color: '#3b82f6' }} />
        <span className="chat-tool-name">{toolName || '工具调用'}</span>
        {!expanded && toolInput && (
          <span className="chat-tool-preview">{getInputPreview()}</span>
        )}
        <span className="chat-tool-toggle">{expanded ? '收起' : '展开'}</span>
        {timestamp && <span className="chat-time">{formatTime(timestamp)}</span>}
      </button>
      {expanded && (
        <div className="chat-tool-content">
          {toolInput && (
            <div className="chat-tool-section">
              <div className="chat-tool-section-label">输入参数</div>
              <pre className="chat-tool-code">{toolInput}</pre>
            </div>
          )}
          {toolResult && (
            <div className="chat-tool-section">
              <div className="chat-tool-section-label">执行结果</div>
              <pre className="chat-tool-code">{toolResult}</pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function ChatBubble({ message }: { message: ChatMessage }) {
  const { role, content, timestamp, toolName, toolInput, toolResult } = message;

  if (role === 'thinking') {
    return <ThinkingBlock content={content} timestamp={timestamp} />;
  }

  if (role === 'tool') {
    return <ToolBlock toolName={toolName} toolInput={toolInput} toolResult={toolResult} timestamp={timestamp} />;
  }

  if (role === 'system') {
    return (
      <div className="chat-system-message">
        <InfoCircleOutlined style={{ color: '#94a3b8', fontSize: 12 }} />
        <span>{content}</span>
        {timestamp && <span className="chat-time">{formatTime(timestamp)}</span>}
      </div>
    );
  }

  if (role === 'result') {
    return (
      <div className="chat-result-block">
        <div className="chat-result-header">
          <CheckCircleOutlined style={{ color: '#22c55e' }} />
          <span>执行结果</span>
          {timestamp && <span className="chat-time">{formatTime(timestamp)}</span>}
        </div>
        <div className="chat-result-content">
          <XMarkdown content={content} />
        </div>
      </div>
    );
  }

  const isUser = role === 'user';
  return (
    <div className={`chat-bubble-row ${isUser ? 'chat-bubble-user' : 'chat-bubble-assistant'}`}>
      <div className="chat-avatar">
        {isUser ? <UserOutlined /> : <RobotOutlined />}
      </div>
      <div className="chat-bubble">
        <div className="chat-bubble-content">
          <XMarkdown content={content} />
        </div>
        {timestamp && <div className="chat-bubble-time">{formatTime(timestamp)}</div>}
      </div>
    </div>
  );
}

export function ChatView({ logs, isRunning }: ChatViewProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const messages = parseLogsToMessages(logs);
  const isInitialMount = useRef(true);

  useEffect(() => {
    // 跳过初始挂载时的自动滚动
    if (isInitialMount.current) {
      isInitialMount.current = false;
      return;
    }
    // 只在有新消息时才滚动到底部
    if (messages.length > 0) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages.length]);

  if (messages.length === 0) {
    return (
      <div className="chat-empty">
        {isRunning ? (
          <div className="chat-loading">
            <LoadingOutlined style={{ fontSize: 24, color: 'var(--color-primary)' }} />
            <span>等待AI响应...</span>
          </div>
        ) : (
          <span>暂无对话记录</span>
        )}
      </div>
    );
  }

  return (
    <div className="chat-container">
      <div className="chat-messages">
        {messages.map((msg, idx) => (
          <ChatBubble key={idx} message={msg} />
        ))}
        {isRunning && (
          <div className="chat-typing-indicator">
            <div className="chat-typing-dot" />
            <div className="chat-typing-dot" />
            <div className="chat-typing-dot" />
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>
    </div>
  );
}
