import { useEffect, useRef } from 'react';

/**
 * 连接到后端 WebSocket `/api/events`，实时接收执行事件。
 * 当收到 Finished / ReviewStatusChanged / Sync 事件时触发 onEvent 回调。
 */
export function useExecutionEvents(onEvent?: () => void) {
  const wsRef = useRef<WebSocket | null>(null);
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const url = `${protocol}//${host}/api/events`;

    function connect() {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        console.debug('[ws-events] connected');
      };

      ws.onmessage = (msg) => {
        try {
          const event = JSON.parse(msg.data);
          // 只关注终态和评审状态变更事件，触发刷新
          if (event.type === 'Finished' || event.type === 'ReviewStatusChanged' || event.type === 'Sync') {
            onEventRef.current?.();
          }
        } catch {
          // ignore parse errors
        }
      };

      ws.onclose = () => {
        console.debug('[ws-events] disconnected');
        wsRef.current = null;
        // 断线后 5 秒重连
        setTimeout(connect, 5000);
      };

      ws.onerror = () => {
        ws.close();
      };
    }

    connect();

    return () => {
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, []);
}
