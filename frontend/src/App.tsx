import { useState } from 'react';
import { ConfigProvider, Layout, Spin } from 'antd';
import { AppProvider, useApp } from './hooks/useApp';
import { Sidebar } from './components/Sidebar';
import { TodoList } from './components/TodoList';
import { TodoDetail } from './components/TodoDetail';
import { CreateTagModal } from './components/CreateTagModal';
import { CreateTodoModal } from './components/CreateTodoModal';
import zhCN from 'antd/locale/zh_CN';
import './App.css';

const { Sider, Content } = Layout;

function AppContent() {
  const { state } = useApp();
  const [tagModalOpen, setTagModalOpen] = useState(false);
  const [todoModalOpen, setTodoModalOpen] = useState(false);

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

  return (
    <Layout style={{ height: '100vh' }}>
      <Sider width={220} style={{ background: '#fff' }}>
        <Sidebar onOpenTagModal={() => setTagModalOpen(true)} />
      </Sider>
      <Layout>
        <Content style={{ display: 'flex' }}>
          <TodoList onOpenCreateModal={() => setTodoModalOpen(true)} />
          <TodoDetail />
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
