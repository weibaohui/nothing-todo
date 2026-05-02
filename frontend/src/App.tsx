import { useState, useEffect } from 'react';
import { ConfigProvider, Layout, Spin, App as AntApp } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { AppProvider, useApp } from './hooks/useApp';
import { useExecutionEvents } from './hooks/useExecutionEvents';
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
    if (todoId) {
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

const customTheme = {
  token: {
    colorPrimary: '#0891b2',
    colorSuccess: '#22c55e',
    colorWarning: '#f59e0b',
    colorError: '#ef4444',
    colorInfo: '#3b82f6',
    borderRadius: 12,
    borderRadiusLG: 16,
    borderRadiusSM: 8,
    fontFamily: "'JetBrains Mono', 'SF Mono', 'Cascadia Code', monospace",
    fontSize: 14,
    controlHeight: 40,
    lineHeight: 1.5,
    colorBgContainer: '#ffffff',
    colorBgLayout: '#f8fafc',
    colorText: '#0f172a',
    colorTextSecondary: '#475569',
    colorBorder: '#e2e8f0',
    colorBorderSecondary: '#f1f5f9',
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.08)',
    boxShadowSecondary: '0 8px 24px rgba(0, 0, 0, 0.12)',
  },
  components: {
    Button: {
      borderRadius: 10,
      controlHeight: 40,
      paddingInline: 20,
    },
    Card: {
      borderRadius: 16,
      paddingLG: 24,
    },
    Modal: {
      borderRadiusLG: 16,
      paddingContentHorizontalLG: 24,
    },
    Input: {
      borderRadius: 10,
      paddingInline: 14,
    },
    Select: {
      borderRadius: 10,
    },
    Tag: {
      borderRadius: 6,
    },
    Switch: {
      colorPrimary: '#0891b2',
    },
  },
};

function App() {
  return (
    <ConfigProvider
      locale={zhCN}
      theme={customTheme}
    >
      <AntApp>
        <AppProvider>
          <AppContent />
        </AppProvider>
      </AntApp>
    </ConfigProvider>
  );
}

export default App;
