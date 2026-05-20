import { useState, useEffect } from 'react';
import { ConfigProvider, Layout, Spin, App as AntApp } from 'antd';
import { PlusOutlined, ThunderboltOutlined, CloseOutlined } from '@ant-design/icons';
import { HashRouter, Routes, Route, useParams, useNavigate, useLocation } from 'react-router-dom';
import { AppProvider, useApp } from './hooks/useApp';
import { useExecutionEvents } from './hooks/useExecutionEvents';
import { ThemeProvider, useTheme } from './hooks/useTheme';
import { TodoList } from './components/TodoList';
import { TodoDetail } from './components/TodoDetail';
import { Dashboard } from './components/Dashboard';
import { MemorialBoard } from './components/MemorialBoard';
import { SettingsPage } from './components/SettingsPage';
import { ExecutionPanel } from './components/ExecutionPanel';
import { TodoDrawer } from './components/TodoDrawer';
import { SmartCreateModal } from './components/SmartCreateModal';
import * as db from './utils/database';
import type { Config } from './types';
import zhCN from 'antd/locale/zh_CN';
import './App.css';

const { Content } = Layout;

const MOBILE_BREAKPOINT = 768;

/** 路由同步 Hook：将 URL 参数同步到 AppContext */
function useRouteSync() {
  const { dispatch, clearSelection } = useApp();
  const location = useLocation();

  useEffect(() => {
    // 从 URL 路径解析状态
    const path = location.pathname;
    const todoMatch = path.match(/^\/todo\/(\d+)/);

    if (todoMatch) {
      dispatch({ type: 'SELECT_TODO', payload: Number(todoMatch[1]) });
    } else if (path === '/settings') {
      clearSelection();
    } else if (path === '/memorial') {
      clearSelection();
    } else {
      // / 或其他路径
      clearSelection();
    }
  }, [location.pathname, dispatch, clearSelection]);
}

/** 获取当前路由决定的视图状态 */
function useRouteView() {
  const location = useLocation();
  const path = location.pathname;

  if (path.startsWith('/todo/')) {
    const todoMatch = path.match(/^\/todo\/(\d+)/);
    const executionMatch = path.match(/^\/todo\/\d+\/execution\/(\d+)/);
    return {
      selectedTodoId: todoMatch ? Number(todoMatch[1]) : null,
      activeView: 'dashboard' as const,
      highlightExecutionId: executionMatch ? Number(executionMatch[1]) : null,
    };
  }
  if (path === '/settings') {
    return { selectedTodoId: null, activeView: 'settings' as const, highlightExecutionId: null };
  }
  if (path === '/memorial') {
    return { selectedTodoId: null, activeView: 'memorial' as const, highlightExecutionId: null };
  }
  return { selectedTodoId: null, activeView: 'dashboard' as const, highlightExecutionId: null };
}

/** Todo Detail 路由组件：从 URL 参数获取 todoId */
function TodoDetailRoute({ onBack, highlightExecutionId }: { onBack?: () => void; highlightExecutionId?: number | null }) {
  const params = useParams<{ id: string }>();
  const { state, dispatch } = useApp();
  const todoId = params.id ? Number(params.id) : null;

  // 同步路由参数到 context
  useEffect(() => {
    if (todoId && todoId !== state.selectedTodoId) {
      dispatch({ type: 'SELECT_TODO', payload: todoId });
    }
  }, [todoId, state.selectedTodoId, dispatch]);

  if (!todoId || !state.todos.find(t => t.id === todoId)) {
    return (
      <div className="flex-center" style={{ height: '100%' }}>
        <Spin />
      </div>
    );
  }

  return <TodoDetail onBack={onBack} highlightExecutionId={highlightExecutionId} />;
}

function AppContent() {
  const { state, dispatch } = useApp();
  const navigate = useNavigate();
  const [todoModalOpen, setTodoModalOpen] = useState(false);
  const [smartCreateOpen, setSmartCreateOpen] = useState(false);
  const [fabExpanded, setFabExpanded] = useState(false);
  const [appConfig, setAppConfig] = useState<Config | null>(null);
  const [isMobile, setIsMobile] = useState(false);
  const [panelCollapsed, setPanelCollapsed] = useState(() => {
    try {
      return localStorage.getItem('execution_panel_collapsed') === 'true';
    } catch {
      return false;
    }
  });

  useExecutionEvents();
  useRouteSync();

  const routeView = useRouteView();
  const hasRunningTasks = Object.keys(state.runningTasks).length > 0;
  const panelHeight = hasRunningTasks ? (panelCollapsed ? 40 : 280) : 0;

  // 移动端：有 todo 选中或非 dashboard 视图时显示详情面板
  const showDetailPanel = isMobile
    ? (routeView.selectedTodoId !== null || routeView.activeView !== 'dashboard')
    : true;
  const showListPanel = isMobile ? !showDetailPanel : true;

  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth < MOBILE_BREAKPOINT);
    };
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  // 加载配置
  useEffect(() => {
    db.getConfig().then(setAppConfig).catch(() => {});
  }, []);

  if (state.loading) {
    return (
      <div className="flex-center" style={{ height: '100vh' }}>
        <Spin size="large" description="加载中..." />
      </div>
    );
  }

  const handleSelectTodo = (todoId: string | number) => {
    navigate(`/todo/${todoId}`);
  };

  const handleShowMemorial = () => {
    navigate('/memorial');
  };

  const handleShowDashboard = () => {
    navigate('/');
  };

  const handleShowSettings = () => {
    navigate('/settings');
  };

  const handleBackToList = () => {
    navigate('/');
  };

  const handleSmartCreateSubmitted = () => {
    db.getAllTodos().then(todos => {
      dispatch({ type: 'SET_TODOS', payload: todos });
    });
  };

  const handleGoToSettings = () => {
    handleShowSettings();
  };

  const handleFabBackdropClick = () => {
    setFabExpanded(false);
  };

  return (
    <Layout style={{ height: '100vh' }}>
      {/* Mobile FAB Group */}
      {isMobile && showListPanel && (
        <>
          {fabExpanded && (
            <div className="mobile-fab-backdrop" onClick={handleFabBackdropClick} />
          )}
          <div className="mobile-fab-group">
            {fabExpanded && (
              <>
                <div className="mobile-fab-item" style={{ animationDelay: '0ms' }}>
                  <span className="mobile-fab-item-label">智能新建</span>
                  <button
                    className="mobile-fab-item-btn mobile-fab-smart"
                    onClick={() => {
                      setFabExpanded(false);
                      setSmartCreateOpen(true);
                    }}
                    aria-label="智能新建"
                  >
                    <ThunderboltOutlined style={{ fontSize: 20, color: '#fff' }} />
                  </button>
                </div>
                <div className="mobile-fab-item" style={{ animationDelay: '50ms' }}>
                  <span className="mobile-fab-item-label">新建</span>
                  <button
                    className="mobile-fab-item-btn mobile-fab-create"
                    onClick={() => {
                      setFabExpanded(false);
                      setTodoModalOpen(true);
                    }}
                    aria-label="新建任务"
                  >
                    <PlusOutlined style={{ fontSize: 20, color: '#fff' }} />
                  </button>
                </div>
              </>
            )}
            <button
              className={`mobile-fab-main ${fabExpanded ? 'expanded' : ''}`}
              onClick={() => setFabExpanded(!fabExpanded)}
              aria-label={fabExpanded ? '关闭' : '创建任务'}
            >
              {fabExpanded ? (
                <CloseOutlined style={{ fontSize: 22, color: '#fff' }} />
              ) : (
                <PlusOutlined style={{ fontSize: 24, color: '#fff' }} />
              )}
            </button>
          </div>
        </>
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
            className={showListPanel ? 'animate-fade-in' : ''}
            style={{
              width: isMobile ? '100%' : 350,
              flexShrink: 0,
              height: '100%',
              display: showListPanel ? 'block' : 'none',
            }}
          >
            <TodoList
              onOpenCreateModal={() => setTodoModalOpen(true)}
              onOpenSmartCreate={() => setSmartCreateOpen(true)}
              onSelectTodo={handleSelectTodo}
              onShowDashboard={handleShowDashboard}
              onShowMemorial={handleShowMemorial}
              onShowSettings={handleShowSettings}
            />
          </div>

          {/* Detail Panel */}
          <div
            className={showDetailPanel ? 'animate-slide-in-right' : ''}
            style={{
              flex: 1,
              height: '100%',
              overflow: 'hidden',
              display: showDetailPanel ? 'block' : 'none',
            }}
          >
            <Routes>
              <Route path="/todo/:id/execution/:executionId" element={
                <TodoDetailRoute onBack={isMobile ? handleBackToList : undefined} highlightExecutionId={routeView.highlightExecutionId} />
              } />
              <Route path="/todo/:id" element={
                <TodoDetailRoute onBack={isMobile ? handleBackToList : undefined} />
              } />
              <Route path="/settings" element={
                <SettingsPage onBack={isMobile ? handleBackToList : undefined} />
              } />
              <Route path="/memorial" element={
                <MemorialBoard onBack={isMobile ? handleBackToList : undefined} />
              } />
              <Route path="/" element={
                <Dashboard onBack={isMobile ? handleBackToList : undefined} />
              } />
            </Routes>
          </div>
        </Content>
      </Layout>

      <TodoDrawer
        open={todoModalOpen}
        todo={null}
        tags={state.tags}
        onClose={() => setTodoModalOpen(false)}
        onSaved={() => {
          db.getAllTodos().then(todos => {
            dispatch({ type: 'SET_TODOS', payload: todos });
          });
        }}
      />

      <SmartCreateModal
        open={smartCreateOpen}
        onClose={() => setSmartCreateOpen(false)}
        isMobile={isMobile}
        config={appConfig}
        onGoToSettings={handleGoToSettings}
        onSubmitted={handleSmartCreateSubmitted}
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
        <HashRouter>
          <AppProvider>
            <AppContent />
          </AppProvider>
        </HashRouter>
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
