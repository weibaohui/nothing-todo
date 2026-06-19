// Loop 阶段面板 (重做: 横向流水线 + 阶段间箭头)。
//
// 对齐参考设计:
// - 阶段卡片横向排列, 中间用 → 连接, 一眼看出执行顺序
// - 每张卡片: 编号 (01/02/03) + 名称 + 描述 + 专家头像 + 执行模式徽章
// - 卡片右下角有 hover 才显示的编辑/删除按钮
// - 「+ 添加阶段」在最右侧 (append 到末尾)
// - 移动顺序: 卡片上的 ← / → 按钮, 比上下移动更符合横向语境

import { useState, useCallback, useEffect } from 'react';
import {
  App as AntApp,
  Button,
  Modal,
  Form,
  Input,
  Select,
  Switch,
  Tag,
  Tooltip,
  Empty,
  Avatar,
  Space,
  InputNumber,
} from 'antd';
import {
  PlusOutlined,
  DeleteOutlined,
  ArrowLeftOutlined,
  ArrowRightOutlined,
  ExperimentOutlined,
  ThunderboltOutlined,
  EditOutlined,
} from '@ant-design/icons';
import * as dbLoops from '@/utils/database/loops';
import * as dbExperts from '@/utils/database/experts';
import * as dbTodos from '@/utils/database/todos';
import type {
  LoopStageDto,
  LoopDetail,
  TodoSummaryForLoop,
  CreateStageRequest,
  UpdateStageRequest,
} from '@/types/loop';
import type { Todo } from '@/types';

interface Props {
  loopId: number;
  stages: LoopStageDto[];
  todoMap: LoopDetail['todo_map'];
  onChanged: () => void;
}

interface StageForm {
  name: string;
  description?: string;
  todo_id: number;
  run_mode: string;
  skip_on_source_failed: boolean;
  min_rating: number | null;
  unrated_policy: string;
  enabled: boolean;
}

interface InlineExpertForm {
  title: string;
  prompt: string;
  executor?: string;
}

// 运行模式 → 中文标签, 当前只有 sequential, 留扩展位
const RUN_MODE_LABEL: Record<string, string> = {
  sequential: '顺序执行',
};

export function LoopStagesPanel({ loopId, stages, todoMap, onChanged }: Props) {
  const { message } = AntApp.useApp();
  const [editing, setEditing] = useState<{ stage: LoopStageDto } | null>(null);
  const [creating, setCreating] = useState(false);
  const [form] = Form.useForm<StageForm>();
  const [candidates, setCandidates] = useState<Todo[]>([]);
  const [candidatesLoading, setCandidatesLoading] = useState(false);
  const [inlineCreateOpen, setInlineCreateOpen] = useState(false);
  const [inlineCreating, setInlineCreating] = useState(false);
  const [inlineForm] = Form.useForm<InlineExpertForm>();

  const reloadCandidates = useCallback(() => {
    setCandidatesLoading(true);
    dbExperts.listExpertCandidates()
      .then(setCandidates)
      .catch(() => setCandidates([]))
      .finally(() => setCandidatesLoading(false));
  }, []);

  useEffect(() => { reloadCandidates(); }, [reloadCandidates]);

  const handleOpenCreate = useCallback(() => {
    form.resetFields();
    form.setFieldsValue({
      name: '',
      description: '',
      todo_id: undefined,
      run_mode: 'sequential',
      skip_on_source_failed: false,
      min_rating: null,
      unrated_policy: 'skip',
      enabled: true,
    });
    setEditing(null);
    setCreating(true);
  }, [form]);

  const handleOpenEdit = useCallback((stage: LoopStageDto) => {
    form.setFieldsValue({
      name: stage.name,
      description: stage.description,
      todo_id: stage.todo_id,
      run_mode: stage.run_mode,
      skip_on_source_failed: stage.skip_on_source_failed,
      min_rating: stage.min_rating,
      unrated_policy: stage.unrated_policy,
      enabled: stage.enabled,
    });
    setEditing({ stage });
    setCreating(false);
  }, [form]);

  const handleClose = useCallback(() => {
    setCreating(false);
    setEditing(null);
  }, []);

  const handleSubmit = useCallback(async () => {
    const values = await form.validateFields();
    if (!values.todo_id) {
      message.error('请选择专家');
      return;
    }
    try {
      if (editing) {
        await dbLoops.updateStage(loopId, editing.stage.id, {
          name: values.name.trim(),
          description: values.description ?? '',
          todo_id: values.todo_id,
          run_mode: values.run_mode,
          skip_on_source_failed: values.skip_on_source_failed,
          min_rating: values.min_rating,
          unrated_policy: values.unrated_policy,
          enabled: values.enabled,
        } as UpdateStageRequest);
        message.success('已更新');
      } else {
        await dbLoops.createStage(loopId, {
          name: values.name.trim(),
          description: values.description ?? '',
          todo_id: values.todo_id,
          run_mode: values.run_mode as 'sequential',
          skip_on_source_failed: values.skip_on_source_failed,
          min_rating: values.min_rating,
          unrated_policy: values.unrated_policy as 'skip',
          enabled: values.enabled,
        } as CreateStageRequest);
        message.success('已添加');
      }
      handleClose();
      onChanged();
    } catch {
      // ignore
    }
  }, [form, editing, loopId, message, handleClose, onChanged]);

  const handleDelete = useCallback(async (id: number) => {
    try {
      await dbLoops.deleteStage(loopId, id);
      message.success('已删除');
      onChanged();
    } catch {
      // ignore
    }
  }, [loopId, message, onChanged]);

  const handleMove = useCallback(async (idx: number, dir: -1 | 1) => {
    const target = idx + dir;
    if (target < 0 || target >= stages.length) return;
    const newOrder = stages.map(s => s.id);
    [newOrder[idx], newOrder[target]] = [newOrder[target], newOrder[idx]];
    try {
      await dbLoops.reorderStages(loopId, { ordered_ids: newOrder });
      onChanged();
    } catch {
      // ignore
    }
  }, [stages, loopId, onChanged]);

  const handleInlineCreateExpert = useCallback(async (values: InlineExpertForm) => {
    if (!values.title?.trim()) {
      message.error('标题必填');
      return;
    }
    setInlineCreating(true);
    try {
      const created: Todo = await dbTodos.createTodo(
        values.title.trim(),
        values.prompt?.trim() ?? '',
        [],
        [],
      );
      await dbExperts.promoteTodoToExpert(created.id);
      message.success(`专家「${created.title}」已创建`);
      setInlineCreateOpen(false);
      inlineForm.resetFields();
      reloadCandidates();
      onChanged();
      form.setFieldValue('todo_id', created.id);
    } catch {
      // ignore
    } finally {
      setInlineCreating(false);
    }
  }, [message, inlineForm, reloadCandidates, onChanged, form]);

  return (
    <div className="loop-stages-panel">
      <div style={{ marginBottom: 12, fontSize: 12, color: 'var(--color-text-secondary, #475569)' }}>
        流水线阶段按从左到右的顺序执行 · 当前 {stages.length} 个阶段
      </div>

      {stages.length === 0 ? (
        <Empty
          description="尚未配置任何阶段, 点右上角「新增阶段」开始"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        >
          <Button type="primary" icon={<PlusOutlined />} onClick={handleOpenCreate}>
            新增阶段
          </Button>
        </Empty>
      ) : (
        <div style={{ display: 'flex', alignItems: 'stretch', overflowX: 'auto', paddingBottom: 8 }}>
          {stages.map((s, idx) => (
            <div key={s.id} style={{ display: 'flex', alignItems: 'center', flexShrink: 0 }}>
              <StageCard
                stage={s}
                index={idx}
                todo={todoMap[s.todo_id]}
                isFirst={idx === 0}
                isLast={idx === stages.length - 1}
                onEdit={() => handleOpenEdit(s)}
                onDelete={() => handleDelete(s.id)}
                onMoveLeft={() => handleMove(idx, -1)}
                onMoveRight={() => handleMove(idx, 1)}
              />
              {/* 阶段之间的箭头 + 末尾 + 按钮 */}
              {idx < stages.length - 1 ? (
                <ArrowConnector />
              ) : (
                <AddStageButton onClick={handleOpenCreate} />
              )}
            </div>
          ))}
          {/* 单阶段时尾巴上也有 + 按钮 (上面 idx < length-1 判断为 false, 单独处理) */}
          {stages.length === 1 && (
            <ArrowConnector />
          )}
        </div>
      )}

      {/* 新建 / 编辑阶段 modal */}
      <Modal
        title={editing ? '编辑阶段' : '新增阶段'}
        open={creating || editing !== null}
        onCancel={handleClose}
        onOk={handleSubmit}
        okText="保存"
        cancelText="取消"
        width={600}
        destroyOnClose
      >
        <Form form={form} layout="vertical">
          <Form.Item label="阶段名称" name="name" rules={[{ required: true, message: '名称必填' }]}>
            <Input placeholder="例如:代码审查" maxLength={100} />
          </Form.Item>
          <Form.Item label="说明" name="description">
            <Input.TextArea rows={2} placeholder="可选" maxLength={500} />
          </Form.Item>
          <Form.Item
            label="目标专家"
            name="todo_id"
            rules={[{ required: true, message: '请选择专家' }]}
            extra={
              <Button
                type="link"
                size="small"
                icon={<ExperimentOutlined />}
                onClick={() => setInlineCreateOpen(true)}
                style={{ padding: 0 }}
              >
                没有合适的专家?内联新建一个
              </Button>
            }
          >
            <Select
              placeholder="选择 kind=expert 的 todo"
              loading={candidatesLoading}
              showSearch
              optionFilterProp="label"
              options={candidates.map(t => ({
                value: t.id,
                label: `#${t.id} ${t.title}${t.executor ? ` (${t.executor})` : ''}`,
              }))}
            />
          </Form.Item>
          <Space style={{ width: '100%' }} size="middle">
            <Form.Item label="执行模式" name="run_mode" style={{ minWidth: 140 }}>
              <Select
                options={[{ value: 'sequential', label: '顺序' }]}
                disabled
              />
            </Form.Item>
            <Form.Item label="评分阈值" name="min_rating" tooltip="前置阶段评分低于此值则跳过">
              <InputNumber min={1} max={5} style={{ width: 100 }} />
            </Form.Item>
            <Form.Item label="未评分策略" name="unrated_policy" style={{ minWidth: 120 }}>
              <Select
                options={[
                  { value: 'skip', label: '跳过' },
                  { value: 'continue', label: '继续' },
                ]}
              />
            </Form.Item>
          </Space>
          <Space size="large">
            <Form.Item label="源阶段失败时跳过" name="skip_on_source_failed" valuePropName="checked">
              <Switch />
            </Form.Item>
            <Form.Item label="启用" name="enabled" valuePropName="checked">
              <Switch />
            </Form.Item>
          </Space>
        </Form>
      </Modal>

      {/* 内联新建专家 modal */}
      <Modal
        title="内联新建专家"
        open={inlineCreateOpen}
        onCancel={() => { setInlineCreateOpen(false); inlineForm.resetFields(); }}
        onOk={() => inlineForm.submit()}
        confirmLoading={inlineCreating}
        okText="创建"
        cancelText="取消"
        destroyOnClose
      >
        <Form
          form={inlineForm}
          layout="vertical"
          onFinish={handleInlineCreateExpert}
          initialValues={{ executor: 'claudecode' }}
        >
          <Form.Item label="标题" name="title" rules={[{ required: true, message: '标题必填' }]}>
            <Input placeholder="例如:代码审查专家" maxLength={100} />
          </Form.Item>
          <Form.Item label="提示词" name="prompt">
            <Input.TextArea rows={4} placeholder="描述这个专家能做什么" maxLength={4000} />
          </Form.Item>
          <Form.Item label="执行器" name="executor">
            <Input placeholder="claudecode" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

// 阶段之间的箭头连接器 (横向 →)
function ArrowConnector() {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      width: 36, flexShrink: 0,
      color: 'var(--color-text-tertiary, #94a3b8)', fontSize: 18,
    }}>
      →
    </div>
  );
}

// 末尾的「+ 添加阶段」按钮 (占位 + 按钮合一)
function AddStageButton({ onClick }: { onClick: () => void }) {
  return (
    <Button
      type="dashed"
      onClick={onClick}
      icon={<PlusOutlined />}
      style={{
        marginLeft: 8,
        height: 120,
        width: 100,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        flexShrink: 0,
      }}
    >
      添加阶段
    </Button>
  );
}

// 单个阶段卡片
function StageCard({ stage, index, todo, isFirst, isLast, onEdit, onDelete, onMoveLeft, onMoveRight }: {
  stage: LoopStageDto;
  index: number;
  todo?: TodoSummaryForLoop;
  isFirst: boolean;
  isLast: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onMoveLeft: () => void;
  onMoveRight: () => void;
}) {
  // 编号用 2 位补零 (01, 02, ...), 与参考设计一致
  const num = String(index + 1).padStart(2, '0');
  const modeLabel = RUN_MODE_LABEL[stage.run_mode] ?? stage.run_mode;

  return (
    <div
      className="loop-stage-card"
      style={{
        position: 'relative',
        width: 200, minHeight: 120,
        background: stage.enabled ? 'var(--color-bg-elevated, #ffffff)' : 'var(--color-bg-hover, #f1f5f9)',
        border: '1px solid var(--color-border, #e2e8f0)',
        borderRadius: 8,
        padding: '10px 12px',
        flexShrink: 0,
        opacity: stage.enabled ? 1 : 0.6,
        transition: 'box-shadow 0.15s',
      }}
    >
      {/* 顶部: 编号 + 名称 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 6 }}>
        <span style={{
          display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
          width: 22, height: 22, borderRadius: 4,
          background: 'var(--color-primary, #0891b2)', color: '#fff', fontSize: 11, fontWeight: 600,
        }}>{num}</span>
        <span style={{
          fontSize: 13, fontWeight: 600, flex: 1, minWidth: 0,
          overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
        }}>{stage.name}</span>
      </div>

      {/* 描述 */}
      {stage.description && (
        <div style={{
          fontSize: 11, color: 'var(--color-text-secondary, #475569)', marginBottom: 8,
          display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical',
          overflow: 'hidden',
        }}>{stage.description}</div>
      )}

      {/* 专家头像 + 名称 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginBottom: 8, minHeight: 24 }}>
        {todo ? (
          <>
            <Avatar size={20} style={{ background: 'var(--color-primary, #0891b2)', fontSize: 10, flexShrink: 0 }}>
              {todo.title.slice(0, 1)}
            </Avatar>
            <Tooltip title={todo.title}>
              <span style={{
                fontSize: 11, color: 'var(--color-primary, #0891b2)', fontWeight: 500, flex: 1, minWidth: 0,
                overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
              }}>{todo.title}</span>
            </Tooltip>
          </>
        ) : (
          <Tag color="red" style={{ margin: 0, fontSize: 11 }}>todo 缺失</Tag>
        )}
      </div>

      {/* 底部 meta: 执行模式 + 执行器 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 10 }}>
        <Tag color="blue" style={{ margin: 0, fontSize: 10, padding: '0 4px' }}>{modeLabel}</Tag>
        {stage.todo_executor && (
          <Tag icon={<ThunderboltOutlined />} color="orange" style={{ margin: 0, fontSize: 10, padding: '0 4px' }}>
            {stage.todo_executor}
          </Tag>
        )}
      </div>

      {/* hover 才显示的操作按钮: 左移 / 右移 / 编辑 / 删除 */}
      <div style={{
        position: 'absolute', top: 6, right: 6,
        display: 'flex', gap: 2,
        opacity: 0.6,
      }}>
        <Tooltip title="左移">
          <Button size="small" type="text" icon={<ArrowLeftOutlined />} disabled={isFirst} onClick={onMoveLeft} />
        </Tooltip>
        <Tooltip title="右移">
          <Button size="small" type="text" icon={<ArrowRightOutlined />} disabled={isLast} onClick={onMoveRight} />
        </Tooltip>
        <Tooltip title="编辑">
          <Button size="small" type="text" icon={<EditOutlined />} onClick={onEdit} />
        </Tooltip>
        <Tooltip title="删除">
          <Button size="small" type="text" danger icon={<DeleteOutlined />} onClick={onDelete} />
        </Tooltip>
      </div>
    </div>
  );
}