import { useEffect, useMemo, useState } from 'react';
import { Button, Empty, Form, Input, InputNumber, Modal, Popconfirm, Select, Switch, Tag, Tooltip } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined, HolderOutlined, StarFilled } from '@ant-design/icons';
import {
  HOOK_TRIGGERS,
  UNRATED_POLICIES,
  DEFAULT_MIN_RATING,
  DEFAULT_UNRATED_POLICY,
  type TodoHookItem,
  type UnratedPolicy,
} from '@/utils/database/hooks';
import type { Todo } from '@/types';

function nextId(): number {
  return Date.now() + Math.floor(Math.random() * 1000);
}

export interface TodoHooksEditorProps {
  /** All todos in the system — used for the "exclude self" filter on the target picker. */
  todos: Todo[];
  /** The todo that owns these hooks. Used to exclude self from the target list. */
  ownerId: number | null;
  /** Current hook list. Controlled by the parent (the create/edit form). */
  hooks: TodoHookItem[];
  /** Called whenever the user adds, edits, deletes, or toggles a hook. */
  onChange: (next: TodoHookItem[]) => void;
  /** Disable all add/edit/delete/toggle controls while the parent is saving. */
  disabled?: boolean;
}

export function TodoHooksEditor({ todos, ownerId, hooks, onChange, disabled }: TodoHooksEditorProps) {
  const [editing, setEditing] = useState<{ open: boolean; item: TodoHookItem | null }>({
    open: false,
    item: null,
  });

  const grouped = useMemo(
    () =>
      HOOK_TRIGGERS.map((t) => ({
        trigger: t,
        items: hooks.filter((h) => h.trigger === t.value),
      })),
    [hooks],
  );

  const handleAdd = (): void => setEditing({ open: true, item: null });
  const handleEdit = (item: TodoHookItem): void => setEditing({ open: true, item });
  const handleDelete = (id: number): void => onChange(hooks.filter((h) => h.id !== id));
  const handleToggle = (id: number, enabled: boolean): void =>
    onChange(hooks.map((h) => (h.id === id ? { ...h, enabled } : h)));
  const handleSubmit = (item: TodoHookItem): void => {
    const exists = hooks.some((h) => h.id === item.id);
    onChange(
      exists ? hooks.map((h) => (h.id === item.id ? item : h)) : [...hooks, item],
    );
    setEditing({ open: false, item: null });
  };

  const targetOptions = todos
    .filter((t) => t.id !== ownerId)
    .map((t) => ({ value: t.id, label: `#${t.id} ${t.title}` }));

  return (
    <div className="detail-card" style={{ marginBottom: 12 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
        <h4 style={{ margin: 0, fontSize: 14, fontWeight: 700, display: 'flex', alignItems: 'center', gap: 6 }}>
          <HolderOutlined /> Hooks
        </h4>
        <Button size="small" type="primary" icon={<PlusOutlined />} onClick={handleAdd} disabled={disabled}>
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
                  const target = todos.find((t) => t.id === item.target_todo_id);
                  const missing = !target;
                  const hasGate = typeof item.min_rating === 'number';
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
                        disabled={disabled}
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
                      {hasGate && (
                        <Tooltip
                          title={
                            item.unrated_policy === 'pass'
                              ? `评分≥${item.min_rating} 才触发；未评分时仍触发`
                              : `评分≥${item.min_rating} 才触发；未评分时不触发`
                          }
                        >
                          <Tag
                            color="gold"
                            icon={<StarFilled />}
                            style={{ margin: 0, fontSize: 11, lineHeight: '16px' }}
                            data-testid="hook-rating-gate"
                          >
                            ≥{item.min_rating} · {item.unrated_policy === 'pass' ? '未评通过' : '未评跳过'}
                          </Tag>
                        </Tooltip>
                      )}
                      <Button
                        size="small"
                        type="text"
                        icon={<EditOutlined />}
                        onClick={() => handleEdit(item)}
                        disabled={disabled}
                        aria-label="编辑 Hook"
                      />
                      <Popconfirm title="删除此 Hook？" onConfirm={() => handleDelete(item.id)} okText="删除" cancelText="取消">
                        <Button
                          size="small"
                          type="text"
                          danger
                          icon={<DeleteOutlined />}
                          disabled={disabled}
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
        targetOptions={targetOptions}
        onCancel={() => setEditing({ open: false, item: null })}
        onOk={handleSubmit}
      />
    </div>
  );
}

interface HookFormValues {
  id: number;
  trigger: TodoHookItem['trigger'];
  target_todo_id: number;
  skip_if_missing: boolean;
  enabled: boolean;
  /** Number | null — `null` means the gate is disabled (no rating requirement). */
  min_rating: number | null;
  unrated_policy: UnratedPolicy;
}

function HookEditModal({
  open,
  item,
  targetOptions,
  onCancel,
  onOk,
}: {
  open: boolean;
  item: TodoHookItem | null;
  targetOptions: { value: number; label: string }[];
  onCancel: () => void;
  onOk: (item: TodoHookItem) => void;
}) {
  const [form] = Form.useForm<HookFormValues>();
  const seedId = useMemo(() => nextId(), [open]);
  // Watch the min_rating field so we can show/hide the unrated_policy
  // control. A rating gate is only meaningful when a threshold is set.
  const minRating = Form.useWatch('min_rating', form);

  useEffect(() => {
    if (!open) return;
    if (item) {
      form.setFieldsValue({
        id: item.id,
        trigger: item.trigger,
        target_todo_id: item.target_todo_id,
        skip_if_missing: item.skip_if_missing ?? true,
        enabled: item.enabled,
        min_rating: typeof item.min_rating === 'number' ? item.min_rating : null,
        unrated_policy: item.unrated_policy ?? DEFAULT_UNRATED_POLICY,
      });
    } else {
      form.setFieldsValue({
        id: seedId,
        trigger: 'state_changed_to_completed',
        target_todo_id: undefined,
        skip_if_missing: true,
        enabled: true,
        min_rating: DEFAULT_MIN_RATING,
        unrated_policy: DEFAULT_UNRATED_POLICY,
      });
    }
  }, [open, item, form, seedId]);

  const handleOk = async (): Promise<void> => {
    const values = await form.validateFields();
    // Normalize the gate fields: if the user left `min_rating` empty, drop
    // the gate entirely (and lock unrated_policy to its default) so we don't
    // send ambiguous `null` payloads that the backend would have to guess at.
    const hasGate = typeof values.min_rating === 'number';
    const next: TodoHookItem = {
      ...values,
      id: item?.id ?? seedId,
      min_rating: hasGate ? values.min_rating : null,
      unrated_policy: hasGate ? values.unrated_policy : DEFAULT_UNRATED_POLICY,
    };
    onOk(next);
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
        <Form.Item
          name="min_rating"
          label="最低评分（0-100，可选）"
          extra={<>留空表示不门控，源 todo 状态变更时总是触发。</>}
        >
          <InputNumber
            min={0}
            max={100}
            step={1}
            precision={0}
            placeholder="留空 = 不门控"
            style={{ width: '100%' }}
            aria-label="最低评分"
          />
        </Form.Item>
        {typeof minRating === 'number' && (
          <Form.Item
            name="unrated_policy"
            label="未评分时"
            extra={(
              <Form.Item
                noStyle
                shouldUpdate={(prev, curr) => prev.unrated_policy !== curr.unrated_policy}
              >
                {({ getFieldValue }) => {
                  const policy = (getFieldValue('unrated_policy') as UnratedPolicy | undefined) ?? DEFAULT_UNRATED_POLICY;
                  return UNRATED_POLICIES.find((p) => p.value === policy)?.description;
                }}
              </Form.Item>
            )}
          >
            <Select options={UNRATED_POLICIES.map((p) => ({ value: p.value, label: p.label }))} />
          </Form.Item>
        )}
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
        💡 目标 todo 的 prompt 作为模板执行；源 todo 的执行结果将作为
        <code>{'{{message}}'}</code> 注入（未执行过则用其 prompt 兜底）。
      </div>
    </Modal>
  );
}
