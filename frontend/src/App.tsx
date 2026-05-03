import { useState, useEffect } from 'react';
import { ConfigProvider, Layout, Spin, App as AntApp } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { AppProvider, useApp } from './hooks/useApp';
import { useExecutionEvents } from './hooks/useExecutionEvents';
import { ThemeProvider, useTheme } from './hooks/useTheme';
import { TodoList } from './components/TodoList';
import { TodoDetail } from './components/TodoDetail';
import { Dashboard } from './components/Dashboard';
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
  const [activeView, setActiveView] = useState<'dashboard' | 'settings'>('dashboard');
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

  const handleSelectTodo = (todoId: string | number | null) => {
    if (todoId != null) {
      setSelectedPanel('detail');
    }
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
              <TodoDetail />
            ) : activeView === 'settings' ? (
              <SettingsPage />
            ) : (
              <Dashboard onBack={isMobile ? () => {
                clearSelection();
                setSelectedPanel('list');
              } : undefined} />
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
