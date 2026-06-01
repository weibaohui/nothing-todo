import { useEffect, useMemo, useState } from 'react';
import { Button, Empty, Form, Input, Modal, Popconfirm, Select, Switch, App } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined, HolderOutlined } from '@ant-design/icons';
import { HOOK_TRIGGERS, type TodoHookItem } from '../../utils/database/hooks';
import * as db from '../../utils/database';
import { useApp } from '../../hooks/useApp';
import type { Todo } from '../../types';

function nextId(): number {
  return Date.now() + Math.floor(Math.random() * 1000);
}

export function TodoHooksEditor({ todo }: { todo: Todo }) {
  const { state, dispatch } = useApp();
  const { message } = App.useApp();
  const hooks = todo.hooks ?? [];
  const [editing, setEditing] = useState<{ open: boolean; item: TodoHookItem | null }>({
    open: false,
    item: null,
  });
  const [saving, setSaving] = useState(false);

  const grouped = useMemo(
    () =>
      HOOK_TRIGGERS.map((t) => ({
        trigger: t,
        items: hooks.filter((h) => h.trigger === t.value),
      })),
    [hooks],
  );

  const save = async (next: TodoHookItem[]): Promise<void> => {
    setSaving(true);
    try {
      const updated = await db.updateTodo(
        todo.id,
        todo.title,
        todo.prompt,
        todo.status,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        next,
      );
      dispatch({ type: 'UPDATE_TODO', payload: updated });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      message.error('Hook 保存失败: ' + msg);
    } finally {
      setSaving(false);
    }
  };

  const handleAdd = (): void => {
    setEditing({ open: true, item: null });
  };

  const handleEdit = (item: TodoHookItem): void => {
    setEditing({ open: true, item });
  };

  const handleDelete = (id: number): void => {
    void save(hooks.filter((h) => h.id !== id));
  };

  const handleToggle = (id: number, enabled: boolean): void => {
    void save(hooks.map((h) => (h.id === id ? { ...h, enabled } : h)));
  };

  const handleSubmit = (item: TodoHookItem): void => {
    const exists = hooks.some((h) => h.id === item.id);
    const next = exists ? hooks.map((h) => (h.id === item.id ? item : h)) : [...hooks, item];
    void save(next);
    setEditing({ open: false, item: null });
  };

  const targetOptions = state.todos
    .filter((t) => t.id !== todo.id)
    .map((t) => ({ value: t.id, label: `#${t.id} ${t.title}` }));

  return (
    <div className="detail-card" style={{ marginBottom: 12 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
        <h4 style={{ margin: 0, fontSize: 14, fontWeight: 700, display: 'flex', alignItems: 'center', gap: 6 }}>
          <HolderOutlined /> Hooks
        </h4>
        <Button size="small" type="primary" icon={<PlusOutlined />} onClick={handleAdd} disabled={saving}>
          添加 Hook
        </Button>
      </div>
      {hooks.length === 0 ? (
        <Empty
          description={<span style={{ color: 'var(--color-text-tertiary)' }}>未配置 Hook</span>}
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          style={{ margin: '12px 0' }}
        />
      ) : (
        <div>
          {grouped.map(({ trigger, items }) =>
            items.length === 0 ? null : (
              <div key={trigger.value} style={{ marginBottom: 10 }}>
                <div
                  style={{
                    fontSize: 11,
                    color: 'var(--color-text-tertiary)',
                    fontWeight: 600,
                    marginBottom: 4,
                    textTransform: 'uppercase',
                    letterSpacing: 0.4,
                  }}
                >
                  {trigger.label}
                </div>
                {items.map((item) => {
                  const target = state.todos.find((t) => t.id === item.target_todo_id);
                  const missing = !target;
                  return (
                    <div
                      key={item.id}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        padding: '6px 8px',
                        border: '1px solid var(--color-border)',
                        borderRadius: 4,
                        marginBottom: 4,
                        opacity: item.enabled ? 1 : 0.5,
                      }}
                    >
                      <Switch
                        size="small"
                        checked={item.enabled}
                        onChange={(c) => handleToggle(item.id, c)}
                        disabled={saving}
                      />
                      <span style={{ flex: 1, fontSize: 13, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {missing ? (
                          <span style={{ color: 'var(--color-error)' }}>
                            #{item.target_todo_id} (已删除{item.skip_if_missing ? ' · 跳过' : ''})
                          </span>
                        ) : (
                          <span>→ {target!.title}</span>
                        )}
                      </span>
                      <Button
                        size="small"
                        type="text"
                        icon={<EditOutlined />}
                        onClick={() => handleEdit(item)}
                        disabled={saving}
                        aria-label="编辑 Hook"
                      />
                      <Popconfirm title="删除此 Hook？" onConfirm={() => handleDelete(item.id)} okText="删除" cancelText="取消">
                        <Button
                          size="small"
                          type="text"
                          danger
                          icon={<DeleteOutlined />}
                          disabled={saving}
                          aria-label="删除 Hook"
                        />
                      </Popconfirm>
                    </div>
                  );
                })}
              </div>
            ),
          )}
        </div>
      )}
      <HookEditModal
        open={editing.open}
        item={editing.item}
        todoId={todo.id}
        targetOptions={targetOptions}
        onCancel={() => setEditing({ open: false, item: null })}
        onOk={handleSubmit}
      />
    </div>
  );
}

function HookEditModal({
  open,
  item,
  todoId,
  targetOptions,
  onCancel,
  onOk,
}: {
  open: boolean;
  item: TodoHookItem | null;
  todoId: number;
  targetOptions: { value: number; label: string }[];
  onCancel: () => void;
  onOk: (item: TodoHookItem) => void;
}) {
  const [form] = Form.useForm<TodoHookItem>();
  const seedId = useMemo(() => nextId(), [open]);

  useEffect(() => {
    if (!open) return;
    if (item) {
      form.setFieldsValue(item);
    } else {
      form.setFieldsValue({
        id: seedId,
        trigger: 'after_create',
        target_todo_id: undefined,
        skip_if_missing: true,
        enabled: true,
      });
    }
  }, [open, item, form, seedId]);

  const handleOk = async (): Promise<void> => {
    const values = await form.validateFields();
    onOk({ ...values, id: item?.id ?? seedId });
  };

  return (
    <Modal
      title={item ? '编辑 Hook' : '添加 Hook'}
      open={open}
      onOk={() => {
        void handleOk();
      }}
      onCancel={onCancel}
      okText="保存"
      cancelText="取消"
      destroyOnClose
    >
      <Form form={form} layout="vertical" preserve={false}>
        <Form.Item name="trigger" label="触发时机" rules={[{ required: true, message: '请选择触发时机' }]}>
          <Select options={HOOK_TRIGGERS.map((t) => ({ value: t.value, label: t.label }))} />
        </Form.Item>
        <Form.Item
          name="target_todo_id"
          label="目标 Todo"
          rules={[{ required: true, message: '请选择要触发的目标 todo' }]}
        >
          <Select
            showSearch
            optionFilterProp="label"
            placeholder={targetOptions.length === 0 ? '没有其他 todo 可选' : '选择 todo'}
            options={targetOptions}
          />
        </Form.Item>
        <Form.Item name="skip_if_missing" label="目标不存在时跳过" valuePropName="checked">
          <Switch />
        </Form.Item>
        <Form.Item name="enabled" label="启用" valuePropName="checked">
          <Switch />
        </Form.Item>
        <Form.Item name="id" hidden>
          <Input type="hidden" />
        </Form.Item>
      </Form>
      <div
        style={{
          fontSize: 12,
          color: 'var(--color-text-tertiary)',
          background: 'var(--color-bg-subtle)',
          padding: 10,
          borderRadius: 4,
          lineHeight: 1.5,
        }}
      >
        💡 触发时，目标 todo <strong>自己的 prompt</strong> 会作为模板执行；
        源 todo 最近一次成功执行的 <strong>结果（result）</strong>会作为
        <code>{'{{message}}'}</code> 自动注入，从而把"上一个 todo 的产出"
        串到下一个 todo 的输入里。源 todo 还没跑过则用它的 prompt 兜底。
        想引用源 todo 的内容？在目标 todo 的 prompt 里写
        <code>{' {{message}} '}</code> 即可。
      </div>
      <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginTop: 8 }}>
        Hook 链路上游 todo：{todoId}
      </div>
    </Modal>
  );
}
