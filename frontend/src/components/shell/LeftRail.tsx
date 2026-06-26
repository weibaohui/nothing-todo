import { useMemo } from 'react';
import type { ReactNode } from 'react';
import { Button, Tooltip, Popover } from 'antd';
import type { ButtonProps } from 'antd';
import {
  InboxOutlined,
  ApartmentOutlined,
  DashboardOutlined,
  ReadOutlined,
  SettingOutlined,
  LaptopOutlined,
  ThunderboltOutlined,
  PlayCircleOutlined,
  FolderOutlined,
  DoubleRightOutlined,
  DoubleLeftOutlined,
} from '@ant-design/icons';
import { WorkspaceSelect } from '@/components/common/WorkspaceSelect';

export type LeftRailKey =
  | 'inbox'
  | 'loops'
  | 'dashboard'
  | 'memorial'
  | 'settings'
  | 'settings_projectDirectories'
  | 'settings_sessions'
  | 'settings_skills'
  | 'settings_runtime';

interface LeftRailItem {
  key: LeftRailKey;
  label: string;
  icon: ReactNode;
  ariaLabel: string;
  danger?: boolean;
}

export type LeftRailVariant = 'rail' | 'drawer';

interface LeftRailProps {
  activeKey: LeftRailKey;
  onSelect: (key: LeftRailKey) => void;
  variant?: LeftRailVariant;
  collapsed?: boolean;
  onToggleCollapsed?: () => void;
  workspace?: string | null;
  onWorkspaceChange?: (workspace: string) => void;
}

/**
 * 左侧主导航栏。
 * 目标：为“中间列表 + 右侧工作区”补上一层全局导航，让用户能用更低成本在核心区域间切换。
 */
export function LeftRail({
  activeKey,
  onSelect,
  variant = 'rail',
  collapsed = true,
  onToggleCollapsed,
  workspace,
  onWorkspaceChange,
}: LeftRailProps) {
  const sections = useMemo(() => ([
    {
      title: '收件箱',
      items: [
        { key: 'inbox', label: '收件箱', icon: <InboxOutlined />, ariaLabel: '收件箱' },
        { key: 'loops', label: '环路', icon: <ApartmentOutlined />, ariaLabel: '环路' },
      ] satisfies LeftRailItem[],
    },
    {
      title: '工作区',
      items: [
        { key: 'dashboard', label: '仪表盘', icon: <DashboardOutlined />, ariaLabel: '仪表盘' },
        { key: 'memorial', label: '看板', icon: <ReadOutlined />, ariaLabel: '看板' },
      ] satisfies LeftRailItem[],
    },
    {
      title: '配置',
      items: [
        { key: 'settings_runtime', label: '运行管理', icon: <PlayCircleOutlined />, ariaLabel: '运行管理' },
        { key: 'settings_skills', label: 'Skills', icon: <ThunderboltOutlined />, ariaLabel: 'Skills' },
        { key: 'settings_projectDirectories', label: '工作空间', icon: <FolderOutlined />, ariaLabel: '工作空间' },
        { key: 'settings_sessions', label: '会话', icon: <LaptopOutlined />, ariaLabel: '会话' },
        { key: 'settings', label: '设置', icon: <SettingOutlined />, ariaLabel: '设置' },
      ] satisfies LeftRailItem[],
    },
  ]), []);

  const isDrawer = variant === 'drawer';
  const shouldShowLabels = isDrawer || !collapsed;

  /**
   * 渲染单个导航按钮。
   * rail：只展示图标（靠 Tooltip 告知含义）；drawer：展示图标 + 文本，适配移动端。
   */
  const renderNavButton = (item: LeftRailItem) => {
    const isActive = item.key === activeKey;
    const commonProps: ButtonProps = {
      type: 'text',
      icon: item.icon,
      onClick: () => onSelect(item.key),
      className: isDrawer ? 'ntd-left-rail-drawer-btn' : 'ntd-left-rail-btn',
      'aria-label': item.ariaLabel,
      'data-testid': `left-rail-${item.key}`,
      danger: item.danger,
    };

    if (isDrawer) {
      return (
        <Button
          key={item.key}
          {...commonProps}
          className={`${commonProps.className} ${isActive ? 'active' : ''}`}
        >
          <span className="ntd-left-rail-drawer-label" data-testid={`left-rail-label-${item.key}`}>{item.label}</span>
        </Button>
      );
    }

    if (!shouldShowLabels) {
      return (
        <Tooltip key={item.key} title={item.label} placement="right">
          <Button
            {...commonProps}
            className={`${commonProps.className} ${isActive ? 'active' : ''}`}
          />
        </Tooltip>
      );
    }

    return (
      <Button
        key={item.key}
        {...commonProps}
        className={`ntd-left-rail-expanded-btn ${isActive ? 'active' : ''}`}
      >
        <span className="ntd-left-rail-expanded-label" data-testid={`left-rail-label-${item.key}`}>{item.label}</span>
      </Button>
    );
  };

  const renderWorkspaceArea = () => {
    if (isDrawer || shouldShowLabels) {
      return (
        <div className={isDrawer ? 'ntd-left-rail-drawer-workspace' : 'ntd-left-rail-workspace'}>
          <div className="ntd-left-rail-workspace-label">工作空间</div>
          <WorkspaceSelect
            value={workspace ?? null}
            required
            onChange={(next) => {
              if (!next) return;
              onWorkspaceChange?.(next);
            }}
            selectProps={{ size: 'small' }}
          />
          <div className="ntd-left-rail-workspace-actions">
            <Button
              type="text"
              size="small"
              icon={<SettingOutlined />}
              onClick={() => onSelect('settings_projectDirectories')}
              aria-label="管理工作空间"
              data-testid="left-rail-manage-workspaces"
            >
              管理
            </Button>
          </div>
        </div>
      );
    }

    return (
      <div className="ntd-left-rail-workspace-collapsed">
        <Popover
          placement="rightTop"
          trigger="click"
          content={
            <div style={{ width: 260, padding: 10 }}>
              <div style={{ fontWeight: 600, marginBottom: 8 }}>工作空间</div>
              <WorkspaceSelect
                value={workspace ?? null}
                required
                onChange={(next) => {
                  if (!next) return;
                  onWorkspaceChange?.(next);
                }}
                selectProps={{ size: 'small' }}
              />
              <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 10 }}>
                <Button
                  type="text"
                  size="small"
                  icon={<SettingOutlined />}
                  onClick={() => onSelect('settings_projectDirectories')}
                  aria-label="管理工作空间"
                >
                  管理
                </Button>
              </div>
            </div>
          }
        >
          <Button
            type="text"
            className="ntd-left-rail-workspace-chip"
            icon={<ApartmentOutlined />}
            aria-label="切换工作空间"
            data-testid="left-rail-workspace"
          />
        </Popover>
      </div>
    );
  };

  return (
    <div
      className={isDrawer ? 'ntd-left-rail-drawer' : `ntd-left-rail ${shouldShowLabels ? 'expanded' : 'collapsed'}`}
      data-testid="left-rail"
    >
      {renderWorkspaceArea()}

      <div className={isDrawer ? 'ntd-left-rail-drawer-top' : 'ntd-left-rail-top'}>
        {sections.map(section => (
          <div key={section.title} className={isDrawer ? 'ntd-left-rail-drawer-section' : 'ntd-left-rail-section'}>
            {shouldShowLabels && (
              <div className={isDrawer ? 'ntd-left-rail-drawer-section-title' : 'ntd-left-rail-section-title'}>
                {section.title}
              </div>
            )}
            <div className={isDrawer ? 'ntd-left-rail-drawer-section-body' : 'ntd-left-rail-section-body'}>
              {section.items
                .filter(it => shouldShowLabels ? true : !String(it.key).startsWith('settings_'))
                .map(renderNavButton)}
            </div>
          </div>
        ))}
      </div>

      {!isDrawer && (
        <div className="ntd-left-rail-bottom">
          <Tooltip title={shouldShowLabels ? '收起导航' : '展开导航'} placement="right">
            <Button
              type="text"
              className="ntd-left-rail-toggle"
              icon={shouldShowLabels ? <DoubleLeftOutlined /> : <DoubleRightOutlined />}
              onClick={onToggleCollapsed}
              aria-label={shouldShowLabels ? '收起导航' : '展开导航'}
              data-testid="left-rail-toggle"
            />
          </Tooltip>
        </div>
      )}
    </div>
  );
}
