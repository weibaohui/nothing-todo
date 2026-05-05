import { useState, useEffect, useMemo } from 'react';
import {
  Card,
  Table,
  Tag,
  Spin,
  Empty,
  Space,
  Typography,
  Tooltip,
  Badge,
  Button,
  Checkbox,
  message,
  Input,
  Select,
  Statistic,
  Row,
  Col,
} from 'antd';
import {
  ThunderboltOutlined,
  SwapOutlined,
  BarChartOutlined,
  AppstoreOutlined,
  CheckCircleOutlined,
  CopyOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import * as db from '../utils/database';
import type { ExecutorSkills, SkillComparison, SkillInvocation } from '../types';
import { EXECUTORS } from '../types';

const { Text, Paragraph } = Typography;

// Executor color map
const EXECUTOR_COLORS: Record<string, string> = {};
EXECUTORS.forEach(e => { EXECUTOR_COLORS[e.value] = e.color; });

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatTime(iso: string | null): string {
  if (!iso) return '-';
  try {
    const d = new Date(iso);
    return d.toLocaleDateString('zh-CN') + ' ' + d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
  } catch {
    return iso;
  }
}

// ── Sub-view: Skills Overview by Executor ──────────────────────────────

function SkillsOverview() {
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState<ExecutorSkills[]>([]);

  useEffect(() => {
    setLoading(true);
    db.getSkillsList()
      .then(setData)
      .catch(err => message.error('加载失败: ' + err.message))
      .finally(() => setLoading(false));
  }, []);

  const totalSkills = useMemo(() => data.reduce((sum, e) => sum + e.skills.length, 0), [data]);
  const executorsWithSkills = useMemo(() => data.filter(e => e.skills.length > 0).length, [data]);

  if (loading) {
    return <div style={{ textAlign: 'center', padding: 48 }}><Spin size="large" /></div>;
  }

  const isMobile = typeof window !== 'undefined' && window.innerWidth < 640;

  return (
    <div>
      <div style={{ display: 'flex', gap: 8, marginBottom: 20, overflowX: 'auto' }}>
        {[
          { title: 'Skill 总数', value: totalSkills, prefix: <ThunderboltOutlined /> },
          { title: '有 Skills 的执行器', value: executorsWithSkills, suffix: `/ ${data.length}` },
          { title: '执行器总数', value: data.length },
        ].map((s, i) => (
          <Card key={i} size="small" style={{ textAlign: 'center', flex: '1 1 0', minWidth: 100 }}>
            <Statistic title={s.title} value={s.value} prefix={s.prefix} suffix={s.suffix} />
          </Card>
        ))}
      </div>

      {data.map(executor => {
        const color = EXECUTOR_COLORS[executor.executor] || '#0891b2';

        return (
          <Card
            key={executor.executor}
            size="small"
            title={
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{
                  width: 8, height: 8, borderRadius: '50%',
                  backgroundColor: executor.skills_dir_exists ? color : '#d9d9d9',
                  flexShrink: 0,
                }} />
                <span style={{ fontWeight: 600 }}>{executor.executor_label}</span>
                <Badge count={executor.skills.length} showZero
                  style={{ backgroundColor: executor.skills.length > 0 ? color : '#d9d9d9' }} />
                {!executor.skills_dir_exists && (
                  <Text type="secondary" style={{ fontSize: 12 }}>- 目录不存在</Text>
                )}
              </div>
            }
            style={{ marginBottom: 8 }}
          >
            {executor.skills_dir_exists && executor.skills.length === 0 && (
              <Empty description="暂无 Skills" image={Empty.PRESENTED_IMAGE_SIMPLE} />
            )}
            {executor.skills.length > 0 && isMobile ? (
              /* Mobile: card list instead of table */
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {executor.skills.map(skill => (
                  <div key={skill.name} style={{
                    padding: '8px 12px',
                    borderRadius: 8,
                    border: '1px solid var(--color-border-light, #f0f0f0)',
                  }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
                      <Text strong style={{ color }}>{skill.name}</Text>
                      {skill.version ? <Tag color={color}>{skill.version}</Tag> : null}
                    </div>
                    {skill.description && (
                      <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 4 }} ellipsis>
                        {skill.description}
                      </Text>
                    )}
                    <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
                      <Text type="secondary" style={{ fontSize: 11 }}>{formatSize(skill.total_size)}</Text>
                      <Text type="secondary" style={{ fontSize: 11 }}>{skill.file_count} 个文件</Text>
                      <Text type="secondary" style={{ fontSize: 11 }}>{formatTime(skill.modified_at)}</Text>
                    </div>
                  </div>
                ))}
              </div>
            ) : executor.skills.length > 0 ? (
              <Table
                dataSource={executor.skills}
                rowKey="name"
                size="small"
                pagination={false}
                scroll={{ x: 600 }}
                columns={[
                  {
                    title: '名称',
                    dataIndex: 'name',
                    width: 160,
                    render: (name: string) => (
                      <Text strong style={{ color }}>{name}</Text>
                    ),
                  },
                  {
                    title: '描述',
                    dataIndex: 'description',
                    ellipsis: true,
                    render: (desc: string) => (
                      <Tooltip title={desc}>
                        <Text type="secondary" ellipsis>{desc || '-'}</Text>
                      </Tooltip>
                    ),
                  },
                  {
                    title: '版本',
                    dataIndex: 'version',
                    width: 80,
                    render: (v: string | null) => v ? <Tag color={color}>{v}</Tag> : '-',
                  },
                  {
                    title: '大小',
                    dataIndex: 'total_size',
                    width: 80,
                    render: (size: number) => formatSize(size),
                  },
                  {
                    title: '文件数',
                    dataIndex: 'file_count',
                    width: 70,
                    render: (n: number) => `${n} 个`,
                  },
                  {
                    title: '更新时间',
                    dataIndex: 'modified_at',
                    width: 120,
                    render: (t: string | null) => <Text type="secondary" style={{ fontSize: 12 }}>{formatTime(t)}</Text>,
                  },
                ]}
              />
            ) : null}
          </Card>
        );
      })}
    </div>
  );
}

// ── Sub-view: Cross-Executor Comparison Matrix ─────────────────────────

function SkillsComparison() {
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState<SkillComparison[]>([]);
  const [filter, setFilter] = useState<'all' | 'shared' | 'unique'>('all');
  const [searchText, setSearchText] = useState('');

  useEffect(() => {
    setLoading(true);
    db.getSkillsComparison()
      .then(setData)
      .catch(err => message.error('加载失败: ' + err.message))
      .finally(() => setLoading(false));
  }, []);

  const filtered = useMemo(() => {
    let result = data;
    if (searchText) {
      const lower = searchText.toLowerCase();
      result = result.filter(s =>
        s.skill_name.toLowerCase().includes(lower) ||
        s.description.toLowerCase().includes(lower)
      );
    }
    if (filter === 'shared') {
      result = result.filter(s => {
        const presentCount = Object.values(s.executors).filter(e => e.present).length;
        return presentCount >= 2;
      });
    } else if (filter === 'unique') {
      result = result.filter(s => {
        const presentCount = Object.values(s.executors).filter(e => e.present).length;
        return presentCount === 1;
      });
    }
    return result;
  }, [data, filter, searchText]);

  const executorColumns = EXECUTORS.map(exec => ({
    title: (
      <Tooltip title={exec.label}>
        <span style={{ fontSize: 12, color: exec.color }}>{exec.label}</span>
      </Tooltip>
    ),
    key: exec.value,
    width: 80,
    align: 'center' as const,
    render: (_: unknown, record: SkillComparison) => {
      const presence = record.executors[exec.value];
      if (!presence?.present) {
        return <span style={{ color: 'var(--color-text-quaternary, #d9d9d9)' }}>-</span>;
      }
      return (
        <Tooltip title={presence.version ? `v${presence.version}` : '已安装'}>
          <CheckCircleOutlined style={{ color: exec.color, fontSize: 16 }} />
        </Tooltip>
      );
    },
  }));

  if (loading) {
    return <div style={{ textAlign: 'center', padding: 48 }}><Spin size="large" /></div>;
  }

  const sharedCount = data.filter(s => Object.values(s.executors).filter(e => e.present).length >= 2).length;
  const uniqueCount = data.filter(s => Object.values(s.executors).filter(e => e.present).length === 1).length;

  return (
    <div>
      <Space style={{ marginBottom: 16 }} wrap>
        <Input.Search
          placeholder="搜索 Skill"
          value={searchText}
          onChange={e => setSearchText(e.target.value)}
          style={{ width: 200 }}
          allowClear
        />
        <Select value={filter} onChange={setFilter} style={{ width: 140 }}>
          <Select.Option value="all">全部 ({data.length})</Select.Option>
          <Select.Option value="shared">共享 ({sharedCount})</Select.Option>
          <Select.Option value="unique">独有 ({uniqueCount})</Select.Option>
        </Select>
      </Space>

      {filtered.length === 0 ? (
        <Empty description="没有匹配的 Skills" />
      ) : (
        <Table
          dataSource={filtered}
          rowKey="skill_name"
          size="small"
          pagination={{ pageSize: 20 }}
          scroll={{ x: 900 }}
          columns={[
            {
              title: 'Skill',
              dataIndex: 'skill_name',
              width: 180,
              fixed: 'left',
              render: (name: string, record: SkillComparison) => {
                const presentCount = Object.values(record.executors).filter(e => e.present).length;
                const totalExecs = EXECUTORS.length;
                let tagColor = 'default';
                let tagLabel = '';
                if (presentCount >= 3) { tagColor = 'green'; tagLabel = '热门'; }
                else if (presentCount >= 2) { tagColor = 'blue'; tagLabel = '共享'; }
                else { tagColor = 'orange'; tagLabel = '独有'; }
                return (
                  <div>
                    <Text strong>{name}</Text>
                    <Tag color={tagColor} style={{ marginLeft: 4, fontSize: 10 }}>{tagLabel}</Tag>
                    <div style={{ marginTop: 2 }}>
                      <Text type="secondary" style={{ fontSize: 11 }}>
                        {presentCount}/{totalExecs} 执行器
                      </Text>
                    </div>
                  </div>
                );
              },
            },
            {
              title: '描述',
              dataIndex: 'description',
              width: 200,
              ellipsis: true,
              render: (desc: string) => (
                <Tooltip title={desc}>
                  <Text type="secondary" ellipsis style={{ fontSize: 12 }}>{desc || '-'}</Text>
                </Tooltip>
              ),
            },
            ...executorColumns,
          ]}
        />
      )}
    </div>
  );
}

// ── Sub-view: Skill Sync ───────────────────────────────────────────────

function SkillSync() {
  const [loading, setLoading] = useState(true);
  const [executors, setExecutors] = useState<ExecutorSkills[]>([]);
  const [selectedExecutor, setSelectedExecutor] = useState<string | null>(null);
  const [selectedSkill, setSelectedSkill] = useState<string | null>(null);
  const [targetExecutors, setTargetExecutors] = useState<string[]>([]);
  const [syncing, setSyncing] = useState(false);
  const [syncResult, setSyncResult] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    db.getSkillsList()
      .then(data => {
        setExecutors(data.filter(e => e.skills_dir_exists));
      })
      .catch(err => message.error('加载失败: ' + err.message))
      .finally(() => setLoading(false));
  }, []);

  const sourceSkills = useMemo(() => {
    if (!selectedExecutor) return [];
    return executors.find(e => e.executor === selectedExecutor)?.skills || [];
  }, [selectedExecutor, executors]);

  const handleSync = async () => {
    if (!selectedExecutor || !selectedSkill || targetExecutors.length === 0) {
      message.warning('请选择源执行器、Skill 和目标执行器');
      return;
    }
    setSyncing(true);
    setSyncResult(null);
    try {
      const result = await db.syncSkill(selectedExecutor, selectedSkill, targetExecutors);
      setSyncResult(result);
      message.success('同步完成');
    } catch (err: any) {
      message.error('同步失败: ' + (err?.message || String(err)));
    } finally {
      setSyncing(false);
    }
  };

  if (loading) {
    return <div style={{ textAlign: 'center', padding: 48 }}><Spin size="large" /></div>;
  }

  return (
    <div style={{ maxWidth: 800 }}>
      <Card title="Skill 同步" size="small" style={{ marginBottom: 16 }}>
        <Paragraph type="secondary" style={{ marginBottom: 16 }}>
          将一个执行器下的 Skill 复制到其他执行器。支持批量同步到多个目标。
        </Paragraph>

        <Space direction="vertical" style={{ width: '100%' }} size="middle">
          <div>
            <Text strong style={{ display: 'block', marginBottom: 8 }}>1. 选择源执行器</Text>
            <Select
              value={selectedExecutor}
              onChange={v => { setSelectedExecutor(v); setSelectedSkill(null); }}
              style={{ width: '100%' }}
              placeholder="选择有 Skills 的执行器"
            >
              {executors.map(e => (
                <Select.Option key={e.executor} value={e.executor}>
                  <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span style={{
                      width: 8, height: 8, borderRadius: '50%',
                      backgroundColor: EXECUTOR_COLORS[e.executor] || '#0891b2',
                    }} />
                    {e.executor_label}
                    <Tag>{e.skills.length} Skills</Tag>
                  </span>
                </Select.Option>
              ))}
            </Select>
          </div>

          {selectedExecutor && (
            <div>
              <Text strong style={{ display: 'block', marginBottom: 8 }}>2. 选择要同步的 Skill</Text>
              <Select
                value={selectedSkill}
                onChange={setSelectedSkill}
                style={{ width: '100%' }}
                placeholder="选择 Skill"
                showSearch
                optionFilterProp="label"
              >
                {sourceSkills.map(s => (
                  <Select.Option key={s.name} value={s.name} label={s.name}>
                    <span>
                      <Text strong>{s.name}</Text>
                      {s.version && <Tag color="blue" style={{ marginLeft: 8 }}>v{s.version}</Tag>}
                      <Text type="secondary" style={{ marginLeft: 8, fontSize: 11 }}>{formatSize(s.total_size)}</Text>
                    </span>
                  </Select.Option>
                ))}
              </Select>
            </div>
          )}

          {selectedSkill && (
            <div>
              <Text strong style={{ display: 'block', marginBottom: 8 }}>3. 选择目标执行器</Text>
              <Checkbox.Group
                value={targetExecutors}
                onChange={v => setTargetExecutors(v as string[])}
                style={{ width: '100%' }}
              >
                <Row gutter={[8, 8]}>
                  {EXECUTORS.filter(e => e.value !== selectedExecutor).map(exec => {
                    const exists = executors.find(ex => ex.executor === exec.value);
                    const alreadyHas = exists?.skills.find(s => s.name === selectedSkill);
                    return (
                      <Col span={12} key={exec.value}>
                        <Checkbox value={exec.value}>
                          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
                            <span style={{
                              width: 6, height: 6, borderRadius: '50%',
                              backgroundColor: exec.color,
                            }} />
                            {exec.label}
                            {alreadyHas && <Tag color="orange" style={{ fontSize: 10 }}>已存在</Tag>}
                          </span>
                        </Checkbox>
                      </Col>
                    );
                  })}
                </Row>
              </Checkbox.Group>
            </div>
          )}

          {syncResult && (
            <div style={{
              padding: 12, borderRadius: 8,
              background: 'var(--color-bg-elevated, #fafafa)',
              border: '1px solid var(--color-border-light, #f0f0f0)',
            }}>
              <Text type="secondary" style={{ fontSize: 12 }}>{syncResult}</Text>
            </div>
          )}

          <div style={{ textAlign: 'right' }}>
            <Button
              type="primary"
              icon={<CopyOutlined />}
              onClick={handleSync}
              loading={syncing}
              disabled={!selectedSkill || targetExecutors.length === 0}
            >
              同步到 {targetExecutors.length} 个执行器
            </Button>
          </div>
        </Space>
      </Card>
    </div>
  );
}

// ── Sub-view: Skill Invocation Tracking ────────────────────────────────

function SkillTracking() {
  const [loading, setLoading] = useState(true);
  const [invocations, setInvocations] = useState<SkillInvocation[]>([]);
  const [page, setPage] = useState(1);
  const [filterSkill, setFilterSkill] = useState<string | undefined>();
  const [filterExecutor, setFilterExecutor] = useState<string | undefined>();

  const loadData = async (p: number, skill?: string, executor?: string) => {
    setLoading(true);
    try {
      const data = await db.getSkillInvocations({
        page: p,
        limit: 20,
        skill_name: skill,
        executor,
      });
      setInvocations(data);
    } catch (err: any) {
      message.error('加载失败: ' + err.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadData(1); }, []);

  const handleRefresh = () => loadData(page, filterSkill, filterExecutor);

  // Stats
  const skillStats = useMemo(() => {
    const map = new Map<string, { count: number; executors: Set<string> }>();
    invocations.forEach(inv => {
      const s = map.get(inv.skill_name) || { count: 0, executors: new Set<string>() };
      s.count++;
      s.executors.add(inv.executor);
      map.set(inv.skill_name, s);
    });
    return Array.from(map.entries())
      .map(([name, data]) => ({ name, count: data.count, executorCount: data.executors.size }))
      .sort((a, b) => b.count - a.count);
  }, [invocations]);

  const isMobile = typeof window !== 'undefined' && window.innerWidth < 640;

  return (
    <div>
      {skillStats.length > 0 && (
        <div style={{ display: 'flex', gap: 8, marginBottom: 16, overflowX: 'auto' }}>
          {skillStats.slice(0, isMobile ? 3 : 4).map(stat => (
            <Card key={stat.name} size="small" style={{ textAlign: 'center', flex: '1 1 0', minWidth: 80 }}>
              <Statistic
                title={<Text ellipsis style={{ maxWidth: isMobile ? 80 : 120, fontSize: 12 }}>{stat.name}</Text>}
                value={stat.count}
                suffix="次"
                valueStyle={{ fontSize: 18 }}
              />
              <Text type="secondary" style={{ fontSize: 10 }}>{stat.executorCount} 个执行器</Text>
            </Card>
          ))}
        </div>
      )}

      <div style={{ display: 'flex', gap: 8, marginBottom: 16, flexWrap: 'wrap' }}>
        <Input.Search
          placeholder="按 Skill 名称筛选"
          allowClear
          style={{ width: isMobile ? '100%' : 200 }}
          onSearch={v => { setFilterSkill(v || undefined); setPage(1); loadData(1, v || undefined, filterExecutor); }}
        />
        <Select
          placeholder="按执行器筛选"
          allowClear
          style={{ width: isMobile ? '100%' : 150 }}
          onChange={v => { setFilterExecutor(v || undefined); setPage(1); loadData(1, filterSkill, v || undefined); }}
        >
          {EXECUTORS.map(e => (
            <Select.Option key={e.value} value={e.value}>{e.label}</Select.Option>
          ))}
        </Select>
        <Button icon={<ReloadOutlined />} onClick={handleRefresh}>刷新</Button>
      </div>

      {invocations.length === 0 ? (
        <Empty description="暂无调用记录" />
      ) : isMobile ? (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {invocations.map(inv => {
            const opt = EXECUTORS.find(e => e.value === inv.executor.toLowerCase());
            const statusMap: Record<string, { color: string; label: string }> = {
              invoked: { color: 'processing', label: '已调用' },
              completed: { color: 'success', label: '完成' },
              failed: { color: 'error', label: '失败' },
            };
            const st = statusMap[inv.status] || { color: 'default', label: inv.status };
            return (
              <div key={inv.id} style={{
                padding: '8px 12px',
                borderRadius: 8,
                border: '1px solid var(--color-border-light, #f0f0f0)',
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
                  <Text strong style={{ color: '#0891b2' }}>{inv.skill_name}</Text>
                  <Tag color={opt?.color || 'default'}>{opt?.label || inv.executor}</Tag>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Text type="secondary" style={{ fontSize: 12 }} ellipsis>
                    {inv.todo_title || `Todo #${inv.todo_id}`}
                  </Text>
                  <Tag color={st.color} style={{ fontSize: 11 }}>{st.label}</Tag>
                </div>
                <div style={{ display: 'flex', gap: 12, marginTop: 4 }}>
                  {inv.duration_ms && <Text type="secondary" style={{ fontSize: 11 }}>{(inv.duration_ms / 1000).toFixed(1)}s</Text>}
                  <Text type="secondary" style={{ fontSize: 11 }}>{formatTime(inv.invoked_at)}</Text>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <Table
          dataSource={invocations}
          rowKey="id"
          size="small"
          loading={loading}
          pagination={{
            current: page,
            pageSize: 20,
            onChange: p => { setPage(p); loadData(p, filterSkill, filterExecutor); },
          }}
          columns={[
            {
              title: 'Skill',
              dataIndex: 'skill_name',
              width: 180,
              render: (name: string) => (
                  <Text strong style={{ color: '#0891b2' }}>{name}</Text>
              ),
            },
            {
              title: '执行器',
              dataIndex: 'executor',
              width: 120,
              render: (exec: string) => {
                const opt = EXECUTORS.find(e => e.value === exec.toLowerCase());
                return (
                  <Tag color={opt?.color || 'default'}>
                    {opt?.label || exec}
                  </Tag>
                );
              },
            },
            {
              title: '关联 Todo',
              dataIndex: 'todo_title',
              width: 200,
              ellipsis: true,
              render: (title: string | null, record: SkillInvocation) => (
                <Tooltip title={title || `Todo #${record.todo_id}`}>
                  <Text type="secondary" ellipsis>{title || `Todo #${record.todo_id}`}</Text>
                </Tooltip>
              ),
            },
            {
              title: '状态',
              dataIndex: 'status',
              width: 100,
              render: (status: string) => {
                const map: Record<string, { color: string; label: string }> = {
                  invoked: { color: 'processing', label: '已调用' },
                  completed: { color: 'success', label: '完成' },
                  failed: { color: 'error', label: '失败' },
                };
                const s = map[status] || { color: 'default', label: status };
                return <Tag color={s.color}>{s.label}</Tag>;
              },
            },
            {
              title: '耗时',
              dataIndex: 'duration_ms',
              width: 100,
              render: (ms: number | null) => ms ? `${(ms / 1000).toFixed(1)}s` : '-',
            },
            {
              title: '调用时间',
              dataIndex: 'invoked_at',
              width: 150,
              render: (t: string) => <Text type="secondary" style={{ fontSize: 12 }}>{formatTime(t)}</Text>,
            },
          ]}
        />
      )}
    </div>
  );
}

// ── Main Skills Panel ───────────────────────────────────────────────────

type SubView = 'overview' | 'compare' | 'sync' | 'tracking';

export function SkillsPanel() {
  const [activeView, setActiveView] = useState<SubView>('overview');

  const views: { key: SubView; label: string; icon: React.ReactNode }[] = [
    { key: 'overview', label: 'Skills 总览', icon: <AppstoreOutlined /> },
    { key: 'compare', label: '对比分析', icon: <BarChartOutlined /> },
    { key: 'sync', label: '同步管理', icon: <SwapOutlined /> },
    { key: 'tracking', label: '调用追踪', icon: <ThunderboltOutlined /> },
  ];

  return (
    <div>
      <div style={{
        display: 'flex',
        flexWrap: 'wrap',
        gap: 8,
        marginBottom: 20,
        borderBottom: '1px solid var(--color-border-light, #f0f0f0)',
        paddingBottom: 12,
      }}>
        {views.map(v => (
          <Button
            key={v.key}
            type={activeView === v.key ? 'primary' : 'default'}
            icon={v.icon}
            onClick={() => setActiveView(v.key)}
            style={{ borderRadius: 8, fontSize: 13, padding: '4px 10px' }}
          >
            {v.label}
          </Button>
        ))}
      </div>

      {activeView === 'overview' && <SkillsOverview />}
      {activeView === 'compare' && <SkillsComparison />}
      {activeView === 'sync' && <SkillSync />}
      {activeView === 'tracking' && <SkillTracking />}
    </div>
  );
}
