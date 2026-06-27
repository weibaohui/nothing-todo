import { useState, useEffect } from 'react';
import type { ReactNode } from 'react';
import { Button } from 'antd';
import { MenuFoldOutlined, MenuUnfoldOutlined } from '@ant-design/icons';
import { PageCard } from './common/PageCard';
import { EmptyDetailPlaceholder } from './EmptyDetailPlaceholder';
import { SIDEBAR_WIDTH } from '@/constants';

interface ListDetailPageProps {
  icon: ReactNode;
  title: string;
  extra?: ReactNode;
  listPanel: ReactNode;
  detailPanel: ReactNode | null;
  storageKey?: string;
}

/**
 * 桌面端列表-详情双栏布局组件
 * 左侧为可折叠的列表侧边栏，右侧为详情内容区
 * 移动端逻辑已独立到 mobile/TodoMobilePage 和 mobile/LoopMobilePage
 */
export function ListDetailPage({
  icon,
  title,
  extra,
  listPanel,
  detailPanel,
  storageKey = 'list_detail_sidebar_collapsed',
}: ListDetailPageProps) {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    try {
      return localStorage.getItem(storageKey) === 'true';
    } catch {
      return true;
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(storageKey, String(sidebarCollapsed));
    } catch {}
  }, [sidebarCollapsed, storageKey]);

  const toggleSidebar = () => {
    setSidebarCollapsed(v => !v);
  };

  const headerExtra = (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      {extra}
      <Button
        type="text"
        size="small"
        icon={sidebarCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
        onClick={toggleSidebar}
        style={{ padding: '0 4px' }}
      />
    </div>
  );

  return (
    <PageCard
      icon={icon}
      title={title}
      extra={headerExtra}
      className="list-detail-page-card"
      style={{ height: '100%', flex: 1, minWidth: 0 }}
      contentStyle={{ padding: 0, display: 'flex', flexDirection: 'row', height: 'calc(100% - 43px)' }}
    >
      <div
        className="list-detail-page-sidebar"
        style={{
          width: sidebarCollapsed ? 16 : SIDEBAR_WIDTH.desktop,
          flexShrink: 0,
          height: '100%',
          overflow: 'hidden',
          transition: 'width 0.2s ease',
          cursor: sidebarCollapsed ? 'pointer' : 'ew-resize',
          position: 'relative',
          background: sidebarCollapsed ? 'var(--color-bg-elevated)' : undefined,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
        onClick={sidebarCollapsed ? toggleSidebar : undefined}
        title={sidebarCollapsed ? '点击展开列表' : undefined}
      >
        {!sidebarCollapsed && (
          <div style={{ width: SIDEBAR_WIDTH.desktop, height: '100%', overflow: 'hidden' }}>
            {listPanel}
          </div>
        )}
        {sidebarCollapsed && (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              gap: 4,
              opacity: 0.4,
            }}
          >
            {[...Array(3)].map((_, i) => (
              <div
                key={i}
                style={{
                  width: 4,
                  height: 4,
                  borderRadius: '50%',
                  background: 'var(--color-text-tertiary)',
                }}
              />
            ))}
          </div>
        )}
      </div>

      <div
        className="list-detail-page-right"
        style={{
          flex: 1,
          minWidth: 0,
          height: '100%',
          overflow: 'hidden',
        }}
      >
        {detailPanel ?? <EmptyDetailPlaceholder />}
      </div>
    </PageCard>
  );
}
