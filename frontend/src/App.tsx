import { useState, useEffect } from 'react';
import { ConfigProvider, Layout, Spin, App as AntApp } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { AppProvider, useApp } from './hooks/useApp';
import { useExecutionEvents } from './hooks/useExecutionEvents';
import { ThemeProvider, useTheme } from './hooks/useTheme';
import { TodoList } from './components/TodoList';
import { TodoDetail } from './components/TodoDetail';
import { Dashboard } from './components/Dashboard';
import { MemorialBoard } from './components/MemorialBoard';
import { SettingsPage } from './components/SettingsPage';
import { ExecutionPanel } from './components/ExecutionPanel';
import { CreateTodoModal } from './components/CreateTodoModal';
import zhCN from 'antd/locale/zh_CN';
import './App.css';

const { Content } = Layout;

const MOBILE_BREAKPOINT = 768;

function AppContent() {
  const { state, clearSelection } = useApp();
  const [todoModalOpen, setTodoModalOpen] = useState(false);
  const [isMobile, setIsMobile] = useState(false);
  const [selectedPanel, setSelectedPanel] = useState<'list' | 'detail'>('list');
  const [activeView, setActiveView] = useState<'dashboard' | 'settings' | 'memorial'>('dashboard');
  const [panelCollapsed, setPanelCollapsed] = useState(() => {
    try {
      return localStorage.getItem('execution_panel_collapsed') === 'true';
    } catch {
      return false;
    }
  });

  useExecutionEvents();

  const hasRunningTasks = Object.keys(state.runningTasks).length > 0;
  const panelHeight = hasRunningTasks ? (panelCollapsed ? 40 : 280) : 0;

  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth < MOBILE_BREAKPOINT);
    };
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  if (state.loading) {
    return (
      <div className="flex-center" style={{ height: '100vh' }}>
        <Spin size="large" description="加载中..." />
      </div>
    );
  }

  if (state.backendStatus === 'unavailable') {
    return (
      <div style={{
        fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 16,
      }}>
        <div style={{
          background: 'white',
          borderRadius: 16,
          padding: 48,
          maxWidth: 500,
          textAlign: 'center',
          boxShadow: '0 25px 50px -12px rgba(0,0,0,0.25)',
        }}>
          <h1 style={{ color: '#1a1a2e', marginBottom: 16, fontSize: 28 }}>欢迎使用 ntd</h1>
          <p style={{ color: '#64748b', marginBottom: 24, lineHeight: 1.6 }}>
            请按照以下步骤安装并启动 ntd 后端服务：
          </p>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 16, textAlign: 'left' }}>
            <div style={{
              width: 32, height: 32, borderRadius: '50%',
              background: '#667eea', color: 'white',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              fontWeight: 'bold', flexShrink: 0,
            }}>1</div>
            <div style={{ color: '#334155' }}>安装 ntd</div>
          </div>
          <div style={{
            background: '#1a1a2e',
            color: '#a5f3fc',
            padding: 16,
            borderRadius: 8,
            fontFamily: "'SF Mono', Monaco, monospace",
            fontSize: 14,
            marginBottom: 24,
            wordBreak: 'break-all',
          }}>
            npm install -g @weibaohui/nothing-todo@latest
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 16, textAlign: 'left' }}>
            <div style={{
              width: 32, height: 32, borderRadius: '50%',
              background: '#667eea', color: 'white',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              fontWeight: 'bold', flexShrink: 0,
            }}>2</div>
            <div style={{ color: '#334155' }}>启动 ntd 服务</div>
          </div>
          <div style={{
            background: '#1a1a2e',
            color: '#a5f3fc',
            padding: 16,
            borderRadius: 8,
            fontFamily: "'SF Mono', Monaco, monospace",
            fontSize: 14,
            marginBottom: 24,
            wordBreak: 'break-all',
          }}>
            ntd daemon start
          </div>
          <p style={{ color: '#94a3b8', fontSize: 14, marginTop: 24 }}>
            安装并启动后，刷新页面即可。
          </p>
          <button
            onClick={() => window.location.reload()}
            style={{
              marginTop: 16,
              padding: '8px 24px',
              background: '#667eea',
              color: 'white',
              border: 'none',
              borderRadius: 8,
              cursor: 'pointer',
              fontSize: 14,
            }}
          >
            重新检查
          </button>
        </div>
      </div>
    );
  }

  const handleSelectTodo = (todoId: string | number | null) => {
    if (todoId != null) {
      setSelectedPanel('detail');
    }
  };

  const handleShowMemorial = () => {
    clearSelection();
    setActiveView('memorial');
    setSelectedPanel('detail');
  };

  const handleShowDashboard = () => {
    clearSelection();
    setActiveView('dashboard');
    setSelectedPanel('detail');
  };

  const handleShowSettings = () => {
    clearSelection();
    setActiveView('settings');
    setSelectedPanel('detail');
  };

  const handleBackToList = () => {
    clearSelection();
    setActiveView('dashboard');
    setSelectedPanel('list');
  };

  return (
    <Layout style={{ height: '100vh' }}>
      {/* Mobile FAB */}
      {isMobile && selectedPanel === 'list' && (
        <button
          className="mobile-fab"
          onClick={() => setTodoModalOpen(true)}
          aria-label="新建任务"
        >
          <PlusOutlined style={{ fontSize: 24, color: '#fff' }} />
        </button>
      )}

      <Layout>
        <Content
          style={{
            display: 'flex',
            flexDirection: isMobile ? 'column' : 'row',
            padding: isMobile ? 0 : 16,
            paddingBottom: isMobile ? 0 : 16 + panelHeight,
            gap: isMobile ? 0 : 16,
            height: `calc(100vh - ${panelHeight}px)`,
            overflow: 'hidden',
            transition: 'height 0.3s ease, padding-bottom 0.3s ease',
          }}
        >
          {/* Todo List Panel */}
          <div
            className={(!isMobile || selectedPanel === 'list') ? 'animate-fade-in' : ''}
            style={{
              width: isMobile ? '100%' : 350,
              flexShrink: 0,
              height: '100%',
              display: !isMobile || selectedPanel === 'list' ? 'block' : 'none',
            }}
          >
            <TodoList
              onOpenCreateModal={() => setTodoModalOpen(true)}
              onSelectTodo={handleSelectTodo}
              onShowDashboard={handleShowDashboard}
              onShowMemorial={handleShowMemorial}
              onShowSettings={handleShowSettings}
            />
          </div>

          {/* Detail Panel */}
          <div
            className={(!isMobile || selectedPanel === 'detail') ? 'animate-slide-in-right' : ''}
            style={{
              flex: 1,
              height: '100%',
              overflow: 'hidden',
              display: !isMobile || selectedPanel === 'detail' ? 'block' : 'none',
            }}
          >
            {state.selectedTodoId ? (
              <TodoDetail onBack={isMobile ? handleBackToList : undefined} />
            ) : activeView === 'settings' ? (
              <SettingsPage onBack={isMobile ? handleBackToList : undefined} />
            ) : activeView === 'memorial' ? (
              <MemorialBoard onBack={isMobile ? handleBackToList : undefined} />
            ) : (
              <Dashboard onBack={isMobile ? handleBackToList : undefined} />
            )}
          </div>
        </Content>
      </Layout>

      <CreateTodoModal
        open={todoModalOpen}
        onClose={() => setTodoModalOpen(false)}
      />
      <ExecutionPanel
        collapsed={panelCollapsed}
        onToggleCollapse={() => {
          const next = !panelCollapsed;
          setPanelCollapsed(next);
          try {
            localStorage.setItem('execution_panel_collapsed', String(next));
          } catch {}
        }}
      />
    </Layout>
  );
}

function ThemedApp() {
  const { themeConfig } = useTheme();

  return (
    <ConfigProvider
      locale={zhCN}
      theme={themeConfig}
    >
      <AntApp>
        <AppProvider>
          <AppContent />
        </AppProvider>
      </AntApp>
    </ConfigProvider>
  );
}

function App() {
  return (
    <ThemeProvider>
      <ThemedApp />
    </ThemeProvider>
  );
}

export default App;
