import { Progress } from 'antd';
import type { TodoItem } from '../types';

interface TodoProgressControlProps {
  todoProgress: TodoItem[];
}

export function TodoProgressControl({ todoProgress }: TodoProgressControlProps) {
  if (!todoProgress || todoProgress.length === 0) return null;

  const total = todoProgress.length;
  const completed = todoProgress.filter(t => t.status === 'completed').length;
  const percent = Math.round((completed / total) * 100);

  return (
    <div style={{ marginBottom: 12 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        <Progress
          percent={percent}
          size="small"
          style={{ flex: 1 }}
          format={() => `${completed}/${total}`}
        />
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {todoProgress.map((item, idx) => (
          <div
            key={item.id || idx}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              fontSize: 12,
              color: item.status === 'completed'
                ? 'var(--color-text-tertiary)'
                : item.status === 'in_progress'
                  ? 'var(--color-primary)'
                  : 'var(--color-text-secondary)',
              textDecoration: item.status === 'completed' ? 'line-through' : 'none',
            }}
          >
            <span style={{ flexShrink: 0, width: 14, textAlign: 'center' }}>
              {item.status === 'completed' ? '✓' : item.status === 'in_progress' ? '●' : '○'}
            </span>
            <span>{item.content}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
