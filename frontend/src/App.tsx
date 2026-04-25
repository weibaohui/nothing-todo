import { useState, useEffect } from 'react';
import { ConfigProvider, Layout, Spin, Button } from 'antd';
import { CloseOutlined, PlusOutlined } from '@ant-design/icons';
import { AppProvider, useApp } from './hooks/useApp';
import { TodoList } from './components/TodoList';
import { TodoDetail } from './components/TodoDetail';
import { CreateTagModal } from './components/CreateTagModal';
import { CreateTodoModal } from './components/CreateTodoModal';
import zhCN from 'antd/locale/zh_CN';
import './App.css';

const { Content } = Layout;

const MOBILE_BREAKPOINT = 768;

function AppContent() {
  const { state } = useApp();
  const [tagModalOpen, setTagModalOpen] = useState(false);
  const [todoModalOpen, setTodoModalOpen] = useState(false);
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
        <Content style={{ display: 'flex', flexDirection: isMobile ? 'column' : 'row', padding: isMobile ? 0 : 16, gap: isMobile ? 0 : 16, height: '100vh', overflow: 'hidden' }}>
          {(!isMobile || selectedPanel === 'list') && (
            <div style={{ width: isMobile ? '100%' : '350px', flexShrink: 0, height: '100%' }}>
              <TodoList
                onOpenCreateModal={() => setTodoModalOpen(true)}
                onSelectTodo={handleSelectTodo}
                onOpenTagModal={() => setTagModalOpen(true)}
              />
            </div>
          )}
          {(!isMobile || selectedPanel === 'detail') && (
            <div style={{ flex: 1, height: '100%', overflow: 'hidden' }}>
              <TodoDetail />
            </div>
          )}
        </Content>
      </Layout>

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
