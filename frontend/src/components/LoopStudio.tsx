// Loop Studio 主页面（容器）。
//
// 设计目标：
// - 顶栏：返回 / 新建 loop / 触发 / 复制 / 启用暂停 / 删除
// - 左栏：loop 列表（每行展示状态 / 名称 / trigger+stage 计数 / 最近一次执行状态）
// - 右栏：当前选中 loop 的详情（基础信息 / triggers / stages / hooks / executions）
// - 所有写操作成功后重新拉取对应子资源，保证 UI 与后端一致
// - 与 TodoList 共用 30-line / 拆小函数 / 注释解释意图 的规范

import { useEffect, useState, useCallback } from 'react';
import { Button, App as AntApp, Empty, Skeleton, Space, Modal, Form, Input } from 'antd';
import { LeftOutlined, PlusOutlined, FormOutlined } from '@ant-design/icons';
import * as dbLoops from '@/utils/database/loops';
import type { LoopListItem, CreateLoopRequest, LoopStatus } from '@/types/loop';
import { LoopListPanel } from './LoopStudioListPanel';
import { LoopDetailPanel } from './LoopStudioDetailPanel';

interface LoopStudioProps {
  onBack?: () => void;
}

export function LoopStudio({ onBack }: LoopStudioProps) {
  const { message } = AntApp.useApp();
  // 列表状态：拉取中 / 已加载 / 异常分别用 loading/loops 控制
  const [loops, setLoops] = useState<LoopListItem[]>([]);
  const [loading, setLoading] = useState(true);
  // 当前选中的 loop id, null = 未选中
  const [selectedId, setSelectedId] = useState<number | null>(null);
  // 新建 loop 的 modal 状态
  const [createOpen, setCreateOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [form] = Form.useForm<CreateLoopRequest>();

  // 加载 loop 列表。失败时降级为空数组, 由 Empty 提示用户新建。
  const reloadLoops = useCallback(() => {
    setLoading(true);
    dbLoops.listLoops()
      .then(setLoops)
      .catch(() => setLoops([]))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => { reloadLoops(); }, [reloadLoops]);

  // 选中 loop 时若不在列表里, 重新选第一个
  useEffect(() => {
    if (selectedId === null && loops.length > 0) {
      setSelectedId(loops[0].id);
    }
    // 当前选中项被删时回退到第一个, 避免 detail 面板 stale
    if (selectedId !== null && !loops.some(l => l.id === selectedId)) {
      setSelectedId(loops[0]?.id ?? null);
    }
  }, [loops, selectedId]);

  // 新建 loop: 后端强制 status=draft, 这里只传 name/description
  const handleCreate = useCallback(async (values: CreateLoopRequest) => {
    if (!values.name?.trim()) {
      message.error('名称必填');
      return;
    }
    setCreating(true);
    try {
      const created = await dbLoops.createLoop({
        name: values.name.trim(),
        description: values.description?.trim() ?? '',
        color: values.color ?? '#0891b2',
        icon: values.icon ?? 'loop',
      });
      message.success(`loop「${created.name}」已创建`);
      setCreateOpen(false);
      form.resetFields();
      reloadLoops();
      // 选中新创建的 loop
      setSelectedId(created.id);
    } catch {
      // 错误已由拦截器提示
    } finally {
      setCreating(false);
    }
  }, [form, message, reloadLoops]);

  // 删除 loop (Popconfirm 已二次确认)
  const handleDelete = useCallback(async (id: number) => {
    try {
      await dbLoops.deleteLoop(id);
      message.success('已删除');
      reloadLoops();
    } catch {
      // ignore: interceptor 弹错
    }
  }, [message, reloadLoops]);

  // 复制 loop
  const handleDuplicate = useCallback(async (id: number) => {
    try {
      const dup = await dbLoops.duplicateLoop(id);
      message.success(`已复制为「${dup.name}」`);
      reloadLoops();
      setSelectedId(dup.id);
    } catch {
      // ignore
    }
  }, [message, reloadLoops]);

  // 切换 loop 状态 (draft <-> enabled <-> paused)
  const handleToggleStatus = useCallback(async (loop: LoopListItem) => {
    const next: LoopStatus = loop.status === 'enabled' ? 'paused' : 'enabled';
    try {
      await dbLoops.updateLoopStatus(loop.id, { status: next });
      message.success(`已${next === 'enabled' ? '启用' : '暂停'}`);
      reloadLoops();
    } catch {
      // ignore
    }
  }, [message, reloadLoops]);

  // 手动触发 loop
  const handleTrigger = useCallback(async (id: number) => {
    try {
      const res = await dbLoops.triggerLoop(id);
      message.success(`已触发 (execution #${res.execution_id})`);
      reloadLoops();
    } catch {
      // ignore
    }
  }, [message, reloadLoops]);

  // 当 detail 内部对 stages/triggers/hooks/executions 做了变更后,
  // 通知上层刷新列表(计数 / 最近执行状态会变)
  const handleDetailChanged = useCallback(() => {
    reloadLoops();
  }, [reloadLoops]);

  // 整页用 column flex, 让 body 在 header 之后填满剩余高度
  return (
    <div
      className="loop-studio-page"
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        minHeight: 0,
        background: 'var(--color-bg-layout, #f8fafc)',
      }}
    >
      {/* 顶部 header: 固定高度, 不参与 flex 收缩 */}
      <div
        className="loop-studio-header"
        style={{
          flexShrink: 0,
          padding: '12px 20px',
          background: 'var(--color-bg-elevated, #1e1e2e)',
          borderBottom: '1px solid var(--color-border, #e2e8f0)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          {onBack && (
            <Button
              type="text"
              size="small"
              icon={<LeftOutlined />}
              onClick={onBack}
              aria-label="返回"
            >
              返回
            </Button>
          )}
          <FormOutlined style={{ color: 'var(--color-primary, #0891b2)', fontSize: 18 }} />
          <h2 style={{ margin: 0, fontSize: 18, fontWeight: 600 }}>环路编排</h2>
          <span style={{ color: 'var(--color-text-tertiary, #94a3b8)', fontSize: 13 }}>
            {loops.length} 个 loop
          </span>
          <div style={{ flex: 1 }} />
          <Space>
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={() => setCreateOpen(true)}
            >
              新建 loop
            </Button>
          </Space>
        </div>
      </div>

      {/* 主体: row flex, 左 360px 列表 + 右 1fr 详情 */}
      <div
        className="loop-studio-body"
        style={{
          display: 'flex',
          flex: 1,
          minHeight: 0,
          overflow: 'hidden',
        }}
      >
        {/* 左栏: 固定 360px 宽, 内部纵向 flex 让过滤 tab 固定 + 列表可滚 */}
        <div
          className="loop-studio-list-col"
          style={{
            width: 360,
            flexShrink: 0,
            display: 'flex',
            flexDirection: 'column',
            minHeight: 0,
            background: 'var(--color-bg-elevated, #1e1e2e)',
            borderRight: '1px solid var(--color-border, #e2e8f0)',
          }}
        >
          {loading ? (
            <Skeleton active style={{ padding: 16 }} />
          ) : loops.length === 0 ? (
            <Empty
              description="暂无 loop；点击右上角新建"
              style={{ marginTop: 64 }}
            />
          ) : (
            <LoopListPanel
              loops={loops}
              selectedId={selectedId}
              onSelect={setSelectedId}
            />
          )}
        </div>

        {/* 右栏: 1fr 占满剩余, 内部 detail panel 自身可滚 */}
        <div
          className="loop-studio-detail-col"
          style={{
            flex: 1,
            minWidth: 0,
            display: 'flex',
            flexDirection: 'column',
            minHeight: 0,
            overflow: 'auto',
          }}
        >
          {selectedId !== null ? (
            <LoopDetailPanel
              loopId={selectedId}
              onTrigger={() => handleTrigger(selectedId)}
              onDuplicate={() => handleDuplicate(selectedId)}
              onDelete={() => handleDelete(selectedId)}
              onToggleStatus={() => {
                const loop = loops.find(l => l.id === selectedId);
                if (loop) handleToggleStatus(loop);
              }}
              onChanged={handleDetailChanged}
            />
          ) : (
            <Empty description="请在左侧选择一个 loop" style={{ marginTop: 64 }} />
          )}
        </div>
      </div>

      <Modal
        title="新建 loop"
        open={createOpen}
        onCancel={() => { setCreateOpen(false); form.resetFields(); }}
        onOk={() => form.submit()}
        confirmLoading={creating}
        okText="创建"
        cancelText="取消"
        destroyOnClose
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item label="名称" name="name" rules={[{ required: true, message: '名称必填' }]}>
            <Input placeholder="例如:每日代码审查" maxLength={100} />
          </Form.Item>
          <Form.Item label="描述" name="description">
            <Input.TextArea rows={3} placeholder="说明这个 loop 的用途" maxLength={500} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
