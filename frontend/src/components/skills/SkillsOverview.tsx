import { useState, useEffect, useMemo } from 'react';
import { Spin, Input, Space, Button, Tag, Dropdown, message } from 'antd';
import type { MenuProps } from 'antd';
import {
  ThunderboltOutlined, SearchOutlined,
  DownloadOutlined, ExportOutlined, ImportOutlined,
  AppstoreOutlined, SettingOutlined,
} from '@ant-design/icons';
import { EXECUTORS } from '@/types';
import type { SkillMeta, ExecutorSkills } from '@/types';
import * as db from '@/utils/database';
import { EXECUTOR_COLORS, formatSize } from './helpers';
import { SkillDetailDrawer } from './SkillDetailDrawer';
import { ImportExportModal } from './ImportExportModal';

export function SkillsOverview() {
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState<ExecutorSkills[]>([]);
  const [searchText, setSearchText] = useState('');
  const [filterExecutor, setFilterExecutor] = useState<string>('all');
  const [selectedSkill, setSelectedSkill] = useState<SkillMeta | null>(null);
  const [selectedExecutor, setSelectedExecutor] = useState('');
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [exportModalOpen, setExportModalOpen] = useState(false);
  const [exportMode, setExportMode] = useState<'import' | 'export'>('export');
  const [initialSelectedSkills, setInitialSelectedSkills] = useState<string[] | undefined>(undefined);

  const loadData = () => {
    setLoading(true);
    db.getSkillsList()
      .then(data => {
        setData(data);
        setSelectedExecutor(prev => {
          if (prev && data.some(e => e.executor === prev)) return prev;
          const withSkills = data.find(e => e.skills.length > 0);
          return withSkills?.executor ?? data[0]?.executor ?? '';
        });
      })
      .catch(err => message.error('加载失败: ' + err.message))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleSkillClick = (skill: SkillMeta, executor: string) => {
    setSelectedSkill(skill);
    setSelectedExecutor(executor);
    setDrawerOpen(true);
  };

  const totalSkills = useMemo(() => data.reduce((sum, e) => sum + e.skills.length, 0), [data]);

  const allSkills = useMemo(() => {
    const skills: { skill: SkillMeta; executor: string }[] = [];
    data.forEach(e => {
      e.skills.forEach(s => {
        if (filterExecutor === 'all' || filterExecutor === e.executor) {
          skills.push({ skill: s, executor: e.executor });
        }
      });
    });
    return skills;
  }, [data, filterExecutor]);

  const filteredSkills = useMemo(() => {
    if (!searchText) return allSkills;
    const lower = searchText.toLowerCase();
    return allSkills.filter(
      ({ skill }) =>
        skill.name.toLowerCase().includes(lower) ||
        skill.description?.toLowerCase().includes(lower) ||
        skill.keywords?.some(k => k.toLowerCase().includes(lower))
    );
  }, [allSkills, searchText]);

  const executorTabs = useMemo(() => {
    const tabs = [{ key: 'all', label: '全部', count: totalSkills }];
    data.forEach(e => {
      const label = EXECUTORS.find(x => x.value === e.executor)?.label || e.executor;
      tabs.push({ key: e.executor, label, count: e.skills.length });
    });
    return tabs;
  }, [data, totalSkills]);

  const exportMenuItems: MenuProps['items'] = [
    { key: 'export', icon: <ExportOutlined />, label: '导出选中' },
    { key: 'export-all', icon: <ExportOutlined />, label: '导出全部' },
    { type: 'divider' },
    { key: 'import', icon: <ImportOutlined />, label: '导入' },
  ];

  const handleExportMenuClick: MenuProps['onClick'] = ({ key }) => {
    if (key === 'import') {
      setExportMode('import');
      setInitialSelectedSkills(undefined);
    } else {
      setExportMode('export');
      if (key === 'export-all') {
        const executorData = data.find(e => e.executor === selectedExecutor);
        if (executorData) {
          setInitialSelectedSkills(executorData.skills.map(s => s.name));
        }
      } else {
        setInitialSelectedSkills(undefined);
      }
    }
    setExportModalOpen(true);
  };

  if (loading) {
    return <div style={{ textAlign: 'center', padding: 48 }}><Spin size="large" /></div>;
  }

  return (
    <div>
      {/* Stats row */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(3, 1fr)',
        gap: 12,
        marginBottom: 20,
      }}>
        <StatCard
          icon={<ThunderboltOutlined />}
          iconColor="#0891b2"
          label="Skill 总数"
          value={totalSkills}
        />
        <StatCard
          icon={<AppstoreOutlined />}
          iconColor="#10b981"
          label="执行器"
          value={data.filter(e => e.skills.length > 0).length}
          suffix={`/ ${data.length}`}
        />
        <StatCard
          icon={<SettingOutlined />}
          iconColor="#f59e0b"
          label="文件总数"
          value={allSkills.reduce((sum, { skill }) => sum + skill.file_count, 0)}
        />
      </div>

      {/* Filter & Search bar */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        marginBottom: 16,
        gap: 12,
        flexWrap: 'wrap',
      }}>
        {/* Executor filter pills */}
        <div style={{
          display: 'flex',
          gap: 6,
          flexWrap: 'wrap',
          flex: 1,
        }}>
          {executorTabs.map(tab => {
            const isActive = filterExecutor === tab.key;
            const color = tab.key === 'all' ? '#0891b2' : (EXECUTOR_COLORS[tab.key] || '#6c7086');
            return (
              <button
                key={tab.key}
                onClick={() => setFilterExecutor(tab.key)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 4,
                  padding: '4px 12px',
                  borderRadius: 20,
                  border: `1px solid ${isActive ? color : 'var(--color-border, #313244)'}`,
                  background: isActive ? `${color}20` : 'transparent',
                  color: isActive ? color : 'var(--color-text-secondary, #a6adc8)',
                  cursor: 'pointer',
                  fontSize: 13,
                  fontWeight: isActive ? 500 : 400,
                  transition: 'all 0.2s',
                  whiteSpace: 'nowrap',
                }}
              >
                {tab.label}
                <span style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  minWidth: 18,
                  height: 18,
                  borderRadius: 9,
                  background: isActive ? color : 'var(--color-fill, #45475a)',
                  color: isActive ? '#fff' : 'var(--color-text-secondary, #a6adc8)',
                  fontSize: 11,
                  lineHeight: 1,
                  padding: '0 4px',
                }}>
                  {tab.count}
                </span>
              </button>
            );
          })}
        </div>

        <Space size={8}>
          <Input
            placeholder="搜索 Skills..."
            prefix={<SearchOutlined style={{ color: 'var(--color-text-quaternary, #6c7086)' }} />}
            value={searchText}
            onChange={e => setSearchText(e.target.value)}
            style={{ width: 200, borderRadius: 20 }}
            allowClear
          />
          <Dropdown menu={{ items: exportMenuItems, onClick: handleExportMenuClick }} trigger={['click']}>
            <Button
              type="primary"
              icon={<DownloadOutlined />}
              style={{ borderRadius: 20 }}
            >
              导入/导出
            </Button>
          </Dropdown>
        </Space>
      </div>

      {/* Skill card grid */}
      {filteredSkills.length === 0 ? (
        <div style={{
          textAlign: 'center',
          padding: '60px 20px',
          color: 'var(--color-text-secondary, #a6adc8)',
        }}>
          <AppstoreOutlined style={{ fontSize: 48, marginBottom: 16, opacity: 0.3 }} />
          <div style={{ fontSize: 16 }}>{searchText ? '无匹配结果' : '暂无 Skills'}</div>
        </div>
      ) : (
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
          gap: 12,
        }}>
          {filteredSkills.map(({ skill, executor }) => (
            <SkillCard
              key={`${executor}-${skill.name}`}
              skill={skill}
              executor={executor}
              onClick={() => handleSkillClick(skill, executor)}
            />
          ))}
        </div>
      )}

      <SkillDetailDrawer
        skill={selectedSkill}
        executor={selectedExecutor}
        executorLabel={EXECUTORS.find(e => e.value === selectedExecutor)?.label || selectedExecutor}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        onSyncSuccess={loadData}
        onDeleteSuccess={loadData}
      />

      <ImportExportModal
        open={exportModalOpen}
        mode={exportMode}
        executor={selectedExecutor}
        data={data}
        initialSelectedSkills={initialSelectedSkills}
        onClose={() => {
          setExportModalOpen(false);
          setInitialSelectedSkills(undefined);
        }}
      />
    </div>
  );
}

function StatCard({ icon, iconColor, label, value, suffix }: {
  icon: React.ReactNode;
  iconColor: string;
  label: string;
  value: number;
  suffix?: string;
}) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      padding: '14px 16px',
      borderRadius: 12,
      background: 'var(--color-bg-container, #1e1e2e)',
      border: '1px solid var(--color-border, #313244)',
      transition: 'border-color 0.2s',
    }}
      onMouseEnter={e => {
        e.currentTarget.style.borderColor = iconColor;
      }}
      onMouseLeave={e => {
        e.currentTarget.style.borderColor = 'var(--color-border, #313244)';
      }}
    >
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: 36,
        height: 36,
        borderRadius: 10,
        background: `${iconColor}18`,
        color: iconColor,
        fontSize: 16,
      }}>
        {icon}
      </div>
      <div>
        <div style={{ fontSize: 12, color: 'var(--color-text-secondary, #a6adc8)' }}>{label}</div>
        <div style={{ fontSize: 20, fontWeight: 600, lineHeight: 1.2, color: 'var(--color-text, #cdd6f4)' }}>
          {value}{suffix && <span style={{ fontSize: 13, fontWeight: 400, marginLeft: 2 }}>{suffix}</span>}
        </div>
      </div>
    </div>
  );
}

function SkillCard({ skill, executor, onClick }: {
  skill: SkillMeta;
  executor: string;
  onClick: () => void;
}) {
  const color = EXECUTOR_COLORS[executor] || '#0891b2';
  const initial = skill.name.replace(/.*\//, '').charAt(0).toUpperCase();
  const executorLabel = EXECUTORS.find(e => e.value === executor)?.label || executor;
  const shortName = skill.name.includes('/') ? skill.name.split('/').pop()! : skill.name;
  const category = skill.name.includes('/') ? skill.name.split('/')[0] : null;

  return (
    <div
      onClick={onClick}
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
        padding: 16,
        borderRadius: 12,
        background: 'var(--color-bg-container, #1e1e2e)',
        border: '1px solid var(--color-border, #313244)',
        cursor: 'pointer',
        transition: 'all 0.2s',
        position: 'relative',
        overflow: 'hidden',
      }}
      onMouseEnter={e => {
        e.currentTarget.style.borderColor = color;
        e.currentTarget.style.boxShadow = `0 4px 12px ${color}20`;
        e.currentTarget.style.transform = 'translateY(-2px)';
      }}
      onMouseLeave={e => {
        e.currentTarget.style.borderColor = 'var(--color-border, #313244)';
        e.currentTarget.style.boxShadow = 'none';
        e.currentTarget.style.transform = 'translateY(0)';
      }}
    >
      {/* Top accent line */}
      <div style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        height: 2,
        background: `linear-gradient(90deg, ${color}, ${color}60)`,
        opacity: 0.6,
      }} />

      {/* Header: icon + name + executor tag */}
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 10 }}>
        <div style={{
          width: 36,
          height: 36,
          borderRadius: 10,
          background: `${color}18`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color,
          fontSize: 15,
          fontWeight: 600,
          flexShrink: 0,
        }}>
          {initial}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
            fontSize: 14,
            fontWeight: 500,
            color: 'var(--color-text, #cdd6f4)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {shortName}
          </div>
          <div style={{
            fontSize: 11,
            color: 'var(--color-text-tertiary, #6c7086)',
            marginTop: 2,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {executorLabel}
          </div>
        </div>
      </div>

      {/* Description */}
      {skill.description && (
        <div style={{
          fontSize: 12,
          color: 'var(--color-text-secondary, #a6adc8)',
          lineHeight: 1.5,
          display: '-webkit-box',
          WebkitLineClamp: 2,
          WebkitBoxOrient: 'vertical',
          overflow: 'hidden',
          minHeight: 36,
        }}>
          {skill.description}
        </div>
      )}

      {/* Footer: tags */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        marginTop: 'auto',
        flexWrap: 'wrap',
      }}>
        {category && (
          <Tag style={{
            margin: 0,
            fontSize: 11,
            lineHeight: '18px',
            padding: '0 6px',
            borderRadius: 4,
            background: 'var(--color-fill, #45475a)',
            border: 'none',
            color: 'var(--color-text-secondary, #a6adc8)',
          }}>
            {category}
          </Tag>
        )}
        {skill.version && (
          <Tag style={{
            margin: 0,
            fontSize: 11,
            lineHeight: '18px',
            padding: '0 6px',
            borderRadius: 4,
            background: `${color}18`,
            border: 'none',
            color,
          }}>
            v{skill.version}
          </Tag>
        )}
        <span style={{
          marginLeft: 'auto',
          fontSize: 11,
          color: 'var(--color-text-quaternary, #585b70)',
        }}>
          {formatSize(skill.total_size)}
        </span>
      </div>
    </div>
  );
}
