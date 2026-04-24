import { useState, useEffect } from 'react';
import { ConfigProvider, Layout, Spin, Drawer, Button } from 'antd';
import { MenuOutlined, CloseOutlined, PlusOutlined } from '@ant-design/icons';
import { AppProvider, useApp } from './hooks/useApp';
import { Sidebar } from './components/Sidebar';
import { TodoList } from './components/TodoList';
import { TodoDetail } from './components/TodoDetail';
import { CreateTagModal } from './components/CreateTagModal';
import { CreateTodoModal } from './components/CreateTodoModal';
import zhCN from 'antd/locale/zh_CN';
import './App.css';

const { Sider, Content } = Layout;

const MOBILE_BREAKPOINT = 768;

function AppContent() {
  const { state } = useApp();
  const [tagModalOpen, setTagModalOpen] = useState(false);
  const [todoModalOpen, setTodoModalOpen] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [isMobile, setIsMobile] = useState(false);
  const [selectedPanel, setSelectedPanel] = useState<'list' | 'detail'>('list');

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
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100vh'
      }}>
        <Spin size="large" tip="加载中..." />
      </div>
    );
  }

  const handleSelectTodo = (todoId: string | number | null) => {
    if (todoId) {
      setSelectedPanel('detail');
    }
  };

  const renderMobileHeader = () => (
    <div className="mobile-header">
      <Button
        type="text"
        icon={<MenuOutlined style={{ fontSize: 20 }} />}
        onClick={() => setSidebarOpen(true)}
        className="mobile-menu-btn"
      />
      <span className="mobile-title">
        {selectedPanel === 'list' ? '任务列表' : state.todos.find(t => t.id === state.selectedTodoId)?.title || '详情'}
      </span>
      {selectedPanel === 'detail' && (
        <Button
          type="text"
          icon={<CloseOutlined style={{ fontSize: 20 }} />}
          onClick={() => {
            setSelectedPanel('list');
          }}
        />
      )}
    </div>
  );

  return (
    <Layout style={{ height: '100vh' }}>
      {isMobile && renderMobileHeader()}

      {!isMobile && (
        <Sider width={220} style={{ background: '#fff', boxShadow: '2px 0 8px rgba(0,0,0,0.05)' }}>
          <div className="sidebar-container">
            <Sidebar onOpenTagModal={() => setTagModalOpen(true)} />
          </div>
        </Sider>
      )}

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
        <Content style={{ display: 'flex', flexDirection: isMobile ? 'column' : 'row', padding: isMobile ? 0 : 16, gap: isMobile ? 0 : 16 }}>
          {(!isMobile || selectedPanel === 'list') && (
            <TodoList
              onOpenCreateModal={() => setTodoModalOpen(true)}
              onSelectTodo={handleSelectTodo}
            />
          )}
          {(!isMobile || selectedPanel === 'detail') && (
            <TodoDetail />
          )}
        </Content>
      </Layout>

      <Drawer
        title="标签"
        placement="left"
        onClose={() => setSidebarOpen(false)}
        open={sidebarOpen}
        width={280}
        styles={{ body: { padding: 0 } }}
      >
        <Sidebar onOpenTagModal={() => {
          setSidebarOpen(false);
          setTagModalOpen(true);
        }} />
      </Drawer>

      <CreateTagModal
        open={tagModalOpen}
        onClose={() => setTagModalOpen(false)}
      />
      <CreateTodoModal
        open={todoModalOpen}
        onClose={() => setTodoModalOpen(false)}
      />
    </Layout>
  );
}

function App() {
  return (
    <ConfigProvider locale={zhCN}>
      <AppProvider>
        <AppContent />
      </AppProvider>
    </ConfigProvider>
  );
}

export default App;
