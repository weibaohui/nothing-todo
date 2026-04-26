import { useRef, useEffect } from 'react';
import { useApp } from '../hooks/useApp';
import { getExecutorOption } from '../types';

const logTypeColors: Record<string, string> = {
  info: '#60a5fa',
  text: '#4ade80',
  tool: '#fbbf24',
  step_start: '#c084fc',
  step_finish: '#2dd4bf',
  stdout: '#cbd5e1',
  stderr: '#f87171',
  error: '#ef4444',
  system: '#94a3b8',
  assistant: '#a78bfa',
  user: '#22d3ee',
  result: '#4ade80',
  thinking: '#fb923c',
};

const logTypeLabels: Record<string, string> = {
  info: 'INFO',
  text: 'TEXT',
  tool: 'TOOL',
  step_start: 'START',
  step_finish: 'END',
  stdout: 'OUT',
  stderr: 'ERR',
  error: 'ERROR',
  system: 'SYS',
  assistant: 'ASST',
  user: 'USER',
  result: 'RESULT',
  thinking: 'THINK',
};

interface ExecutionPanelProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
}

export function ExecutionPanel({ collapsed, onToggleCollapse }: ExecutionPanelProps) {
  const { state, dispatch } = useApp();
  const { runningTasks, activeTaskId } = state;
  const logsEndRef = useRef<HTMLDivElement>(null);

  const taskIds = Object.keys(runningTasks);
  const activeTask = activeTaskId ? runningTasks[activeTaskId] : null;

  useEffect(() => {
    if (logsEndRef.current && !collapsed && activeTask) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [activeTask?.logs, collapsed, activeTask]);

  if (taskIds.length === 0) return null;

  return (
    <div className={`execution-panel ${collapsed ? 'collapsed' : ''}`}>
      {/* Tab Bar */}
      <div className="execution-panel-tabs">
        <div className="execution-panel-tabs-scroll">
          {taskIds.map((taskId) => {
            const task = runningTasks[taskId];
            const opt = getExecutorOption(task.executor);
            const isActive = taskId === activeTaskId;
            return (
              <div
                key={taskId}
                className={`execution-tab ${isActive ? 'active' : ''} ${task.status}`}
                onClick={() => dispatch({ type: 'SET_ACTIVE_TASK', payload: taskId })}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    dispatch({ type: 'SET_ACTIVE_TASK', payload: taskId });
                  }
                }}
              >
                <span className="tab-icon">{opt.icon}</span>
                <span className="tab-title" title={task.todoTitle}>
                  {task.todoTitle}
                </span>
                {task.status === 'running' && <span className="tab-spinner" />}
                {task.status === 'finished' && (
                  <span className={`tab-status ${task.success ? 'success' : 'error'}`}>
                    {task.success ? '✓' : '✗'}
                  </span>
                )}
                <button
                  className="tab-close"
                  onClick={(e) => {
                    e.stopPropagation();
                    dispatch({ type: 'REMOVE_RUNNING_TASK', payload: taskId });
                  }}
                  aria-label="关闭"
                >
                  ×
                </button>
              </div>
            );
          })}
        </div>
        <div className="execution-panel-actions">
          <span className="task-count">{taskIds.length} 个任务</span>
          <button
            className="panel-toggle-btn"
            onClick={onToggleCollapse}
            aria-label={collapsed ? '展开' : '收起'}
          >
            {collapsed ? '▲' : '▼'}
          </button>
        </div>
      </div>

      {/* Log Area */}
      {!collapsed && activeTask && (
        <div className="execution-panel-logs">
          {activeTask.logs.length === 0 ? (
            <div className="execution-panel-empty">等待输出...</div>
          ) : (
            <>
              {activeTask.logs.map((log, idx) => (
                <div key={idx} className="log-line">
                  <span className="log-timestamp">{log.timestamp}</span>
                  <span
                    className="log-type-badge"
                    style={{
                      color: logTypeColors[log.type] || '#cbd5e1',
                      background: `${logTypeColors[log.type] || '#cbd5e1'}20`,
                    }}
                  >
                    {logTypeLabels[log.type] || log.type}
                  </span>
                  <span className="log-content">{log.content}</span>
                </div>
              ))}
              {activeTask.status === 'finished' && activeTask.result && (
                <div
                  className={`log-result ${activeTask.success ? 'log-result-success' : 'log-result-error'}`}
                >
                  {activeTask.result}
                </div>
              )}
              <div ref={logsEndRef} />
            </>
          )}
        </div>
      )}
    </div>
  );
}
