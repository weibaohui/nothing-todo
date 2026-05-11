import { useState, useEffect, useCallback } from 'react';
import {
  Table,
  Tag,
  Space,
  Input,
  Select,
  Drawer,
  Button,
  Popconfirm,
  Statistic,
  Card,
  Row,
  Col,
  Spin,
  Empty,
  Typography,
  Tooltip,
  message,
} from 'antd';
import {
  SearchOutlined,
  ReloadOutlined,
  EyeOutlined,
  DeleteOutlined,
  RobotOutlined,
  ClockCircleOutlined,
  ThunderboltOutlined,
  TeamOutlined,
  ApiOutlined,
  FilterOutlined,
} from '@ant-design/icons';
import * as db from '../utils/database';
import type { SessionInfo, SessionDetail, SessionStats } from '../utils/database';

const { Text, Paragraph } = Typography;

// ─── Source display config ────────────────────────────────

const sourceConfig: Record<string, { label: string; color: string; icon?: string }> = {
  'claude-code': { label: 'Claude Code', color: '#d97706' },
  'codex': { label: 'Codex', color: '#10a37f' },
  'hermes': { label: 'Hermes', color: '#8b5cf6' },
  'kimi': { label: 'Kimi', color: '#3b82f6' },
  'atomcode': { label: 'AtomCode', color: '#ef4444' },
  'cc-connect': { label: 'CC-Connect', color: '#06b6d4' },
};

function sourceTag(source: string) {
  const cfg = sourceConfig[source] || { label: source, color: '#6b7280' };
  return (
    <Tag color={cfg.color} style={{ fontSize: 11, lineHeight: '18px', padding: '0 6px' }}>
      {cfg.label}
    </Tag>
  );
}

// ─── Helpers ──────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatTokens(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`;
  return `${(n / 1_000_000).toFixed(2)}M`;
}

function formatTime(iso?: string | null): string {
  if (!iso) return '-';
  try {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);

    if (diffMin < 1) return '刚刚';
    if (diffMin < 60) return `${diffMin} 分钟前`;
    const diffHour = Math.floor(diffMin / 60);
    if (diffHour < 24) return `${diffHour} 小时前`;
    const diffDay = Math.floor(diffHour / 24);
    if (diffDay < 30) return `${diffDay} 天前`;

    return d.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' });
  } catch {
    return iso;
  }
}

function shortId(id: string): string {
  return id.length > 12 ? `${id.slice(0, 8)}...${id.slice(-4)}` : id;
}

// ─── Stats Cards ──────────────────────────────────────────

function StatsCards({ stats }: { stats: SessionStats | null }) {
  if (!stats) return null;

  // Source breakdown row
  const sourceEntries = Object.entries(stats.by_source).sort((a, b) => b[1] - a[1]);

  return (
    <>
      <Row gutter={[12, 12]} style={{ marginBottom: 12 }}>
        <Col span={6}>
          <Card size="small" style={{ textAlign: 'center' }}>
            <Statistic
              title={<Text type="secondary" style={{ fontSize: 12 }}>总会话</Text>}
              value={stats.total_sessions}
              prefix={<TeamOutlined />}
              valueStyle={{ fontSize: 20 }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small" style={{ textAlign: 'center' }}>
            <Statistic
              title={<Text type="secondary" style={{ fontSize: 12 }}>活跃会话</Text>}
              value={stats.active_sessions}
              prefix={<ClockCircleOutlined />}
              valueStyle={{ fontSize: 20, color: '#52c41a' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small" style={{ textAlign: 'center' }}>
            <Statistic
              title={<Text type="secondary" style={{ fontSize: 12 }}>今日新增</Text>}
              value={stats.today_sessions}
              prefix={<ThunderboltOutlined />}
              valueStyle={{ fontSize: 20, color: '#faad14' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small" style={{ textAlign: 'center' }}>
            <Statistic
              title={<Text type="secondary" style={{ fontSize: 12 }}>总 Token</Text>}
              value={formatTokens(stats.total_input_tokens + stats.total_output_tokens)}
              prefix={<ApiOutlined />}
              valueStyle={{ fontSize: 20, color: '#1677ff' }}
            />
          </Card>
        </Col>
      </Row>
      {/* Source breakdown */}
      {sourceEntries.length > 0 && (
        <div style={{ marginBottom: 12, display: 'flex', gap: 6, flexWrap: 'wrap', alignItems: 'center' }}>
          <FilterOutlined style={{ color: 'var(--color-text-secondary)' }} />
          <Text type="secondary" style={{ fontSize: 12 }}>工具分布：</Text>
          {sourceEntries.map(([source, count]) => {
            const cfg = sourceConfig[source] || { label: source, color: '#6b7280' };
            return (
              <Tag key={source} color={cfg.color} style={{ fontSize: 11, margin: 0 }}>
                {cfg.label} {count}
              </Tag>
            );
          })}
        </div>
      )}
    </>
  );
}

// ─── Session Detail Drawer ────────────────────────────────

function SessionDetailDrawer({
  sessionId,
  open,
  onClose,
}: {
  sessionId: string | null;
  open: boolean;
  onClose: () => void;
}) {
  const [detail, setDetail] = useState<SessionDetail | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (open && sessionId) {
      setLoading(true);
      db.getSessionDetail(sessionId)
        .then(setDetail)
        .catch(() => setDetail(null))
        .finally(() => setLoading(false));
    } else {
      setDetail(null);
    }
  }, [open, sessionId]);

  return (
    <Drawer
      title={detail ? (
        <Space>
          {sourceTag(detail.info.source)}
          <span>Session {shortId(detail.info.session_id)}</span>
        </Space>
      ) : 'Session 详情'}
      open={open}
      onClose={onClose}
      width={680}
      styles={{ body: { padding: '16px 24px' } }}
    >
      <Spin spinning={loading}>
        {detail ? (
          <>
            {/* Meta Info */}
            <Card size="small" title="基本信息" style={{ marginBottom: 16 }}>
              <Row gutter={[16, 8]}>
                <Col span={12}><Text type="secondary">工具：</Text>{sourceTag(detail.info.source)}</Col>
                <Col span={12}><Text type="secondary">状态：</Text>
                  <Tag color={detail.info.status === 'active' ? 'green' : 'default'}>
                    {detail.info.status === 'active' ? '活跃' : '已完成'}
                  </Tag>
                </Col>
                <Col span={12}><Text type="secondary">项目：</Text><Text>{detail.info.project_path}</Text></Col>
                <Col span={12}><Text type="secondary">执行器：</Text><Text>{detail.info.executor}</Text></Col>
                <Col span={12}><Text type="secondary">模型：</Text><Text code>{detail.info.model}</Text></Col>
                <Col span={12}><Text type="secondary">Git 分支：</Text><Text code>{detail.info.git_branch || '-'}</Text></Col>
                <Col span={12}><Text type="secondary">版本：</Text><Text code>{detail.info.version || '-'}</Text></Col>
                <Col span={12}><Text type="secondary">消息数：</Text><Text>{detail.info.message_count}</Text></Col>
                <Col span={12}>
                  <Text type="secondary">文件大小：</Text>
                  <Text>{formatBytes(detail.info.file_size)}</Text>
                </Col>
                <Col span={12}>
                  <Text type="secondary">Token：</Text>
                  <Tooltip title={`输入: ${formatTokens(detail.info.total_input_tokens)} / 输出: ${formatTokens(detail.info.total_output_tokens)}`}>
                    <Text>{formatTokens(detail.info.total_input_tokens + detail.info.total_output_tokens)}</Text>
                  </Tooltip>
                </Col>
                <Col span={12}><Text type="secondary">子代理：</Text><Text>{detail.info.subagent_count}</Text></Col>
                <Col span={24}>
                  <Text type="secondary">首条 Prompt：</Text>
                  <Paragraph
                    ellipsis={{ rows: 3, expandable: true, symbol: '展开' }}
                    style={{ marginTop: 4, marginBottom: 0 }}
                  >
                    {detail.info.first_prompt || '-'}
                  </Paragraph>
                </Col>
              </Row>
            </Card>

            {/* Subagents */}
            {detail.subagents.length > 0 && (
              <Card size="small" title={`子代理 (${detail.subagents.length})`} style={{ marginBottom: 16 }}>
                {detail.subagents.map((sa, i) => (
                  <div key={i} style={{ padding: '6px 0', borderBottom: i < detail.subagents.length - 1 ? '1px solid var(--color-border-light)' : 'none' }}>
                    <Space>
                      <Tag color="purple">{sa.agent_type}</Tag>
                      <Text>{sa.description}</Text>
                    </Space>
                  </div>
                ))}
              </Card>
            )}

            {/* Messages Timeline */}
            <Card size="small" title={`对话记录 (${detail.messages.length})`}>
              <div style={{ maxHeight: 500, overflowY: 'auto' }}>
                {detail.messages.length === 0 ? (
                  <Empty description="无对话记录" />
                ) : (
                  detail.messages.map((msg, i) => (
                    <div
                      key={i}
                      style={{
                        marginBottom: 12,
                        padding: '8px 12px',
                        borderRadius: 8,
                        background: msg.role === 'user'
                          ? 'var(--color-bg-elevated)'
                          : 'rgba(22, 119, 255, 0.06)',
                        borderLeft: msg.role === 'user'
                          ? '3px solid var(--color-border)'
                          : '3px solid #1677ff',
                      }}
                    >
                      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                        <Space size={4}>
                          <RobotOutlined style={{ color: msg.role === 'user' ? undefined : '#1677ff' }} />
                          <Text strong style={{ fontSize: 12 }}>
                            {msg.role === 'user' ? '用户' : '助手'}
                          </Text>
                          {msg.model && <Tag color="blue" style={{ fontSize: 10, lineHeight: '16px', padding: '0 4px' }}>{msg.model}</Tag>}
                        </Space>
                        <Space size={8}>
                          {msg.input_tokens != null && (
                            <Text type="secondary" style={{ fontSize: 11 }}>
                              ↑{formatTokens(msg.input_tokens)}
                            </Text>
                          )}
                          {msg.output_tokens != null && (
                            <Text type="secondary" style={{ fontSize: 11 }}>
                              ↓{formatTokens(msg.output_tokens)}
                            </Text>
                          )}
                          <Text type="secondary" style={{ fontSize: 11 }}>{formatTime(msg.timestamp)}</Text>
                        </Space>
                      </div>
                      <Paragraph
                        ellipsis={{ rows: 4, expandable: true, symbol: '展开' }}
                        style={{ margin: 0, fontSize: 13, whiteSpace: 'pre-wrap' }}
                      >
                        {msg.content_preview || '(无内容)'}
                      </Paragraph>
                    </div>
                  ))
                )}
              </div>
            </Card>
          </>
        ) : (
          <Empty description="选择一个 Session 查看详情" />
        )}
      </Spin>
    </Drawer>
  );
}

// ─── Main Component ───────────────────────────────────────

export function SessionManager() {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [stats, setStats] = useState<SessionStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [statusFilter, setStatusFilter] = useState<string | undefined>();
  const [sourceFilter, setSourceFilter] = useState<string | undefined>();
  const [searchText, setSearchText] = useState('');
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const fetchSessions = useCallback(async () => {
    setLoading(true);
    try {
      const res = await db.listSessions({
        page,
        page_size: pageSize,
        status: statusFilter,
        source: sourceFilter,
        search: searchText || undefined,
      });
      setSessions(res.sessions);
      setTotal(res.total);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, [page, pageSize, statusFilter, sourceFilter, searchText]);

  const fetchStats = useCallback(async () => {
    try {
      const s = await db.getSessionStats();
      setStats(s);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    fetchSessions();
  }, [fetchSessions]);

  useEffect(() => {
    fetchStats();
  }, [fetchStats]);

  const handleDelete = async (sessionId: string) => {
    try {
      await db.deleteSession(sessionId);
      message.success('已删除');
      fetchSessions();
      fetchStats();
    } catch (e: any) {
      message.error(e.message || '删除失败');
    }
  };

  // Source filter options from stats
  const sourceOptions = stats
    ? Object.keys(stats.by_source).map((s) => {
        const cfg = sourceConfig[s] || { label: s };
        return { label: `${cfg.label} (${stats.by_source[s]})`, value: s };
      })
    : [];

  const columns = [
    {
      title: '状态',
      dataIndex: 'status',
      width: 60,
      render: (s: string) => (
        <Tooltip title={s === 'active' ? '活跃' : '已完成'}>
          <span
            style={{
              display: 'inline-block',
              width: 8,
              height: 8,
              borderRadius: '50%',
              background: s === 'active' ? '#52c41a' : '#d9d9d9',
              boxShadow: s === 'active' ? '0 0 6px rgba(82, 196, 26, 0.5)' : 'none',
            }}
          />
        </Tooltip>
      ),
    },
    {
      title: '工具',
      dataIndex: 'source',
      width: 120,
      render: (source: string) => sourceTag(source),
    },
    {
      title: 'Session ID',
      dataIndex: 'session_id',
      width: 130,
      render: (id: string) => (
        <Tooltip title={id}>
          <Text code style={{ fontSize: 12 }}>{shortId(id)}</Text>
        </Tooltip>
      ),
    },
    {
      title: '项目',
      dataIndex: 'project_path',
      width: 180,
      ellipsis: true,
      render: (p: string) => {
        const short = p.split('/').slice(-2).join('/');
        return <Tooltip title={p}><Text style={{ fontSize: 12 }}>{short}</Text></Tooltip>;
      },
    },
    {
      title: '模型',
      dataIndex: 'model',
      width: 90,
      ellipsis: true,
      render: (m: string) => <Text style={{ fontSize: 12 }}>{m}</Text>,
    },
    {
      title: '分支',
      dataIndex: 'git_branch',
      width: 85,
      ellipsis: true,
      render: (b: string | null) => b ? <Tag style={{ fontSize: 11 }}>{b}</Tag> : <Text type="secondary">-</Text>,
    },
    {
      title: '消息',
      dataIndex: 'message_count',
      width: 55,
      align: 'center' as const,
      render: (n: number) => <Text style={{ fontSize: 12 }}>{n}</Text>,
    },
    {
      title: 'Token',
      width: 75,
      align: 'right' as const,
      render: (_: unknown, r: SessionInfo) => {
        const total = r.total_input_tokens + r.total_output_tokens;
        if (total === 0) return <Text type="secondary" style={{ fontSize: 12 }}>-</Text>;
        return (
          <Tooltip title={`输入: ${formatTokens(r.total_input_tokens)} / 输出: ${formatTokens(r.total_output_tokens)}`}>
            <Text style={{ fontSize: 12 }}>{formatTokens(total)}</Text>
          </Tooltip>
        );
      },
    },
    {
      title: '首条 Prompt',
      dataIndex: 'first_prompt',
      ellipsis: true,
      render: (p: string | null) => (
        <Text type="secondary" style={{ fontSize: 12 }}>{p || '-'}</Text>
      ),
    },
    {
      title: '最后活跃',
      dataIndex: 'last_active_at',
      width: 110,
      render: (t: string | null) => (
        <Tooltip title={t || ''}>
          <Text style={{ fontSize: 12 }}>{formatTime(t)}</Text>
        </Tooltip>
      ),
    },
    {
      title: '操作',
      width: 70,
      fixed: 'right' as const,
      render: (_: unknown, r: SessionInfo) => (
        <Space size={4}>
          <Button
            type="text"
            size="small"
            icon={<EyeOutlined />}
            onClick={(e) => { e.stopPropagation(); setSelectedSessionId(r.session_id); setDrawerOpen(true); }}
          />
          <Popconfirm
            title="确定删除该 Session？"
            description="将删除会话文件数据（不可恢复）"
            onConfirm={(e) => { e?.stopPropagation(); handleDelete(r.session_id); }}
            okText="删除"
            cancelText="取消"
          >
            <Button type="text" size="small" danger icon={<DeleteOutlined />} onClick={(e) => e.stopPropagation()} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <StatsCards stats={stats} />

      {/* Filters */}
      <div style={{ display: 'flex', gap: 8, marginBottom: 12, flexWrap: 'wrap' }}>
        <Input
          placeholder="搜索 Prompt 内容..."
          prefix={<SearchOutlined />}
          value={searchText}
          onChange={(e) => { setSearchText(e.target.value); setPage(1); }}
          style={{ width: 220 }}
          allowClear
        />
        <Select
          placeholder="工具来源"
          value={sourceFilter}
          onChange={(v) => { setSourceFilter(v); setPage(1); }}
          style={{ width: 170 }}
          allowClear
          options={sourceOptions}
        />
        <Select
          placeholder="状态"
          value={statusFilter}
          onChange={(v) => { setStatusFilter(v); setPage(1); }}
          style={{ width: 110 }}
          allowClear
          options={[
            { label: '活跃', value: 'active' },
            { label: '已完成', value: 'completed' },
          ]}
        />
        <Button icon={<ReloadOutlined />} onClick={() => { fetchSessions(); fetchStats(); }}>
          刷新
        </Button>
      </div>

      {/* Table */}
      <Table
        dataSource={sessions}
        columns={columns}
        rowKey="session_id"
        loading={loading}
        size="small"
        scroll={{ x: 1300 }}
        pagination={{
          current: page,
          pageSize,
          total,
          showSizeChanger: true,
          showTotal: (t) => `共 ${t} 条`,
          onChange: (p, ps) => { setPage(p); setPageSize(ps); },
        }}
        onRow={(record) => ({
          onClick: () => { setSelectedSessionId(record.session_id); setDrawerOpen(true); },
          style: { cursor: 'pointer' },
        })}
      />

      {/* Detail Drawer */}
      <SessionDetailDrawer
        sessionId={selectedSessionId}
        open={drawerOpen}
        onClose={() => { setDrawerOpen(false); setSelectedSessionId(null); }}
      />
    </div>
  );
}
