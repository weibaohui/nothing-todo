import { useState, useEffect } from 'react';
import { Select, Button, List, Empty, Spin, Tag, Popconfirm, message, Modal, Switch, Radio } from 'antd';
import { LinkOutlined, DisconnectOutlined, FolderOutlined, RobotOutlined } from '@ant-design/icons';
import * as db from '@/utils/database';
import { PENDING_CHAT_ID } from '@/utils/database/bots';
import type { AgentBot, FeishuProjectBindingItem } from '@/utils/database/bots';
import type { ProjectDirectory } from '@/utils/database';
import type { Todo } from '@/types';
import { EXECUTORS, RESUMABLE_EXECUTORS } from '@/types/execution';

/** 仅显示支持继续对话的执行器选项 */
const RESUMABLE_EXECUTOR_OPTIONS = EXECUTORS.filter(e => RESUMABLE_EXECUTORS.has(e.value));

/** 项目绑定管理面板 — 管理飞书聊天与项目目录的绑定关系 */
export function ProjectBindsTab() {
  const [bindings, setBindings] = useState<FeishuProjectBindingItem[]>([]);
  const [bots, setBots] = useState<AgentBot[]>([]);
  const [directories, setDirectories] = useState<ProjectDirectory[]>([]);
  const [todos, setTodos] = useState<Todo[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedBotId, setSelectedBotId] = useState<number | undefined>(undefined);
  const [selectedDirId, setSelectedDirId] = useState<number | undefined>(undefined);
  const [selectedExecutor, setSelectedExecutor] = useState<string>('claudecode');
  const [bindToExisting, setBindToExisting] = useState(false);
  const [selectedTodoId, setSelectedTodoId] = useState<number | undefined>(undefined);
  const [bindModalOpen, setBindModalOpen] = useState(false);
  const [binding, setBinding] = useState(false);
  const [selectedBindingId, setSelectedBindingId] = useState<number | undefined>(undefined);

  const loadAll = async () => {
    setLoading(true);
    try {
      const [b, d, t] = await Promise.all([
        db.getAgentBots(),
        db.getProjectDirectories(),
        db.getAllTodos(),
      ]);
      setBots(b.filter(bot => bot.bot_type === 'feishu'));
      setDirectories(d);
      setTodos(t);

      const bindings = selectedBotId !== undefined
        ? await db.getFeishuBindings(selectedBotId)
        : await db.getFeishuBindings();
      setBindings(bindings);

      // Initialize radio selection to the currently-enabled binding
      const activeBinding = bindings.find(binding => binding.enabled);
      if (activeBinding) {
        setSelectedBindingId(activeBinding.id);
      }
    } catch (err: any) {
      message.error('加载数据失败: ' + (err?.message || String(err)));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadAll(); }, []);

  const handleBotChange = async (botId: number | undefined) => {
    setSelectedBotId(botId);
    try {
      if (botId !== undefined) {
        setBindings(await db.getFeishuBindings(botId));
      } else {
        setBindings(await db.getFeishuBindings());
      }
    } catch (err: any) {
      message.error('加载绑定列表失败');
    }
  };

  const resetModal = () => {
    setSelectedDirId(undefined);
    setSelectedExecutor('claudecode');
    setBindToExisting(false);
    setSelectedTodoId(undefined);
  };

  const handleCreateBinding = async () => {
    if (selectedBotId === undefined) {
      message.error('请选择 Bot');
      return;
    }
    if (selectedDirId === undefined) {
      message.error('请选择项目目录');
      return;
    }
    if (bindToExisting && selectedTodoId === undefined) {
      message.error('请选择要绑定的 Todo');
      return;
    }

    setBinding(true);
    try {
      await db.createFeishuBinding({
        bot_id: selectedBotId,
        chat_id: PENDING_CHAT_ID,
        chat_type: 'p2p',
        project_dir_id: selectedDirId,
        executor: bindToExisting ? undefined : selectedExecutor,
        todo_id: bindToExisting ? selectedTodoId : undefined,
      });
      message.success('绑定已创建（请在飞书中使用 /bind 命令绑定具体聊天）');
      setBindModalOpen(false);
      resetModal();
      handleBotChange(selectedBotId);
    } catch (err: any) {
      message.error('创建绑定失败: ' + (err?.message || String(err)));
    } finally {
      setBinding(false);
    }
  };

  // 解绑：仅禁用（enabled=false），保留记录
  const handleUnbindBinding = async (id: number) => {
    try {
      await db.updateFeishuBindingEnabled(id, false);
      message.success('已解绑');
      handleBotChange(selectedBotId);
    } catch (err: any) {
      message.error('解绑失败: ' + (err?.message || String(err)));
    }
  };

  // 删除：彻底删除绑定记录
  const handleDeleteBinding = async (id: number) => {
    try {
      await db.deleteFeishuBinding(id);
      message.success('已删除');
      handleBotChange(selectedBotId);
    } catch (err: any) {
      message.error('删除失败: ' + (err?.message || String(err)));
    }
  };

  const handleToggleEnabled = async (id: number, enabled: boolean) => {
    try {
      await db.updateFeishuBindingEnabled(id, enabled);
      message.success(enabled ? '已启用' : '已禁用');
      handleBotChange(selectedBotId);
    } catch (err: any) {
      message.error((enabled ? '启用' : '禁用') + '失败: ' + (err?.message || String(err)));
    }
  };

  // Radio 选择：启用对应绑定（单选，同一时间只有一个活跃）
  // 选新绑定前先禁用旧绑定，避免数据库 unique 约束冲突
  const handleSelectBinding = async (id: number) => {
    if (selectedBindingId === id) return; // 已是选中状态
    try {
      if (selectedBindingId !== undefined) {
        await db.updateFeishuBindingEnabled(selectedBindingId, false);
      }
      await db.updateFeishuBindingEnabled(id, true);
      setSelectedBindingId(id);
      message.success('已设为活跃绑定');
      handleBotChange(selectedBotId);
    } catch (err: any) {
      message.error('切换绑定失败: ' + (err?.message || String(err)));
    }
  };

  const chatTypeLabel = (t: string) => t === 'p2p' ? '私聊' : '群聊';
  const isPending = (item: FeishuProjectBindingItem) => item.chat_id === PENDING_CHAT_ID;
  const statusTag = (s: string, pending: boolean) => {
    if (pending) return <Tag color="orange">待绑定</Tag>;
    if (s === 'running') return <Tag color="green">运行中</Tag>;
    return <Tag>空闲</Tag>;
  };

  // 过滤出有 workspace 的项目类 Todo，供用户选择绑定到已有 Todo
  const projectTodos = todos.filter(t => t.workspace);

  return (
    <Spin spinning={loading}>
      {/* Bot 选择器 */}
      <div style={{ marginBottom: 16, display: 'flex', gap: 12, alignItems: 'center' }}>
        <div style={{ fontWeight: 500, whiteSpace: 'nowrap' }}>选择 Bot：</div>
        <Select
          placeholder="选择飞书 Bot"
          allowClear
          style={{ width: 280 }}
          value={selectedBotId}
          onChange={handleBotChange}
          options={bots.map(b => ({
            label: `${b.bot_name} (${b.app_id})`,
            value: b.id,
          }))}
        />
        <Button type="primary" icon={<LinkOutlined />} onClick={() => { resetModal(); setBindModalOpen(true); }}>
          新建绑定
        </Button>
      </div>

      {/* 绑定列表 */}
      {bindings.length === 0 ? (
        <Empty description={selectedBotId ? '该 Bot 暂无项目绑定' : '暂无绑定，请先选择 Bot'} image={Empty.PRESENTED_IMAGE_SIMPLE} />
      ) : (
        <List
          dataSource={bindings}
          renderItem={(item) => (
            <List.Item
              style={{
                padding: '12px',
                background: 'var(--color-bg)',
                borderRadius: 6,
                marginBottom: 8,
                border: '1px solid var(--color-border-light)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 12, flex: 1, minWidth: 0 }}>
                {/* Radio 单选：表示该绑定是否被选为活跃绑定 */}
                <Radio
                  checked={selectedBindingId === item.id}
                  onChange={() => handleSelectBinding(item.id)}
                  disabled={isPending(item)}
                  style={{ flexShrink: 0 }}
                />
                <RobotOutlined style={{ fontSize: 18, color: item.enabled ? '#52c41a' : '#999' }} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span style={{ fontWeight: 500 }}>
                      {item.project_name || item.project_path || '(未知项目)'}
                    </span>
                    {statusTag(item.status, isPending(item))}
                    {!item.enabled && !isPending(item) && <Tag color="red">已禁用</Tag>}
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginTop: 2 }}>
                    <FolderOutlined style={{ marginRight: 4 }} />
                    {item.project_path || '(无路径)'}
                    <span style={{ margin: '0 8px' }}>|</span>
                    {isPending(item) ? (
                      <span style={{ color: 'var(--color-warning)' }}>未绑定聊天（使用飞书 /bind 命令绑定）</span>
                    ) : (
                      <>{chatTypeLabel(item.chat_type)} · {item.chat_id}</>
                    )}
                    <span style={{ margin: '0 8px' }}>|</span>
                    Todo #{item.todo_id}
                    {item.session_id && <span> · Session: {item.session_id.slice(0, 8)}…</span>}
                  </div>
                </div>
                {/* 启用/禁用开关（非待绑定状态才显示） */}
                {!isPending(item) && (
                  <Switch
                    size="small"
                    checked={item.enabled}
                    onChange={(checked) => handleToggleEnabled(item.id, checked)}
                    style={{ flexShrink: 0 }}
                  />
                )}
                {/* 解绑：仅禁用（enabled=false），保留记录 */}
                {!isPending(item) && (
                  <Popconfirm
                    title="确认解绑"
                    description={`解除与「${item.project_name || item.project_path}」的绑定？（可随时重新启用）`}
                    onConfirm={() => handleUnbindBinding(item.id)}
                  >
                    <Button type="text" icon={<DisconnectOutlined />} size="small">
                      解绑
                    </Button>
                  </Popconfirm>
                )}
                {/* 删除：彻底删除记录 */}
                <Popconfirm
                  title="确认删除"
                  description={`删除与「${item.project_name || item.project_path}」的绑定记录？此操作不可恢复。`}
                  onConfirm={() => handleDeleteBinding(item.id)}
                >
                  <Button type="text" danger size="small">
                    删除
                  </Button>
                </Popconfirm>
              </div>
            </List.Item>
          )}
        />
      )}

      {/* 新建绑定 Modal */}
      <Modal
        title="新建项目绑定"
        open={bindModalOpen}
        onCancel={() => { setBindModalOpen(false); resetModal(); }}
        onOk={handleCreateBinding}
        confirmLoading={binding}
        okText={bindToExisting ? '绑定' : '创建'}
      >
        <div style={{ marginBottom: 12 }}>选择要绑定的项目目录：</div>
        <Select
          placeholder="选择项目目录"
          style={{ width: '100%', marginBottom: 12 }}
          value={selectedDirId}
          onChange={(v) => { setSelectedDirId(v); setSelectedTodoId(undefined); }}
          options={directories.map(d => ({
            label: `${d.name || '(未命名)'} — ${d.path}`,
            value: d.id,
          }))}
        />

        {/* 绑定已有 Todo 开关 */}
        <div style={{ marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8 }}>
          <Switch
            checked={bindToExisting}
            onChange={(checked) => { setBindToExisting(checked); setSelectedTodoId(undefined); }}
            size="small"
          />
          <span style={{ fontSize: 13, color: 'var(--color-text-secondary)' }}>
            绑定到已有的 Todo（继续之前的对话）
          </span>
        </div>

        {bindToExisting ? (
          <>
            <div style={{ marginBottom: 8, fontSize: 13, color: 'var(--color-text-secondary)' }}>
              选择要绑定的 Todo：
            </div>
            <Select
              placeholder="选择一个 Todo"
              style={{ width: '100%', marginBottom: 12 }}
              value={selectedTodoId}
              onChange={setSelectedTodoId}
              options={projectTodos.map(t => ({
                label: `#${t.id} ${t.title} ${t.workspace ? `· ${t.workspace}` : ''}`,
                value: t.id,
              }))}
            />
          </>
        ) : (
          <>
            <div style={{ marginBottom: 8, fontSize: 13 }}>选择执行器（仅支持继续对话的执行器）：</div>
            <Select
              style={{ width: '100%', marginBottom: 12 }}
              value={selectedExecutor}
              onChange={setSelectedExecutor}
              options={RESUMABLE_EXECUTOR_OPTIONS.map(e => ({
                label: e.label,
                value: e.value,
              }))}
            />
          </>
        )}

        <div style={{ fontSize: 13, color: 'var(--color-text-secondary)' }}>
          创建绑定后，请在对应的飞书聊天中使用 <code>/bind &lt;项目名称&gt;</code> 命令完成绑定。
        </div>
      </Modal>
    </Spin>
  );
}
