// Loop Studio 类型定义。
//
// 与 backend/src/models/loop_.rs 一一对应：
// - LoopDto = 环路主表 DTO
// - LoopTriggerDto = 触发器 DTO (cron / webhook / feishu / manual)
// - LoopStageDto = 阶段 DTO, 含关联 todo 的精简信息
// - LoopHookDto = loop hook DTO (pre_loop / post_loop / pre_stage / post_stage)
// - LoopExecutionDto = 单次执行记录
// - LoopStageExecutionDto = 单个阶段的执行记录
//
// 前端组件用这些类型组装 LoopStudio 页面。

export type LoopStatus = 'draft' | 'enabled' | 'paused';

export type LoopTriggerType =
  | 'manual'
  | 'cron'
  | 'webhook'
  | 'feishu_message'
  | 'feishu_command'
  | 'todo_completed'
  | 'todo_state_changed'
  | 'tag_added';

export type LoopHookPosition = 'pre_loop' | 'post_loop' | 'pre_stage' | 'post_stage';

export type LoopRunMode = 'sequential'; // 当前仅顺序; 留扩展位

export type LoopUnratedPolicy = 'skip' | 'continue';

export type LoopExecutionStatus = 'running' | 'success' | 'partial' | 'failed' | 'cancelled';

export type LoopStageExecutionStatus =
  | 'pending'
  | 'running'
  | 'success'
  | 'failed'
  | 'skipped';

export interface LoopDto {
  id: number;
  name: string;
  description: string;
  product: string;
  repo: string;
  branch: string;
  status: string; // 后端字符串,前端不强枚举,留扩展
  color: string;
  icon: string;
  created_at: string | null;
  updated_at: string | null;
}

export interface LoopTriggerDto {
  id: number;
  loop_id: number;
  trigger_type: string; // 后端字符串灵活
  config: string; // JSON 字符串 (cron 表达式 / webhook_id / matches 等)
  enabled: boolean;
  priority: number;
  created_at: string | null;
}

export interface LoopStageRawDto {
  id: number;
  loop_id: number;
  order_index: number;
  name: string;
  description: string;
  todo_id: number;
  run_mode: string; // 当前仅 sequential
  skip_on_source_failed: boolean;
  min_rating: number | null;
  unrated_policy: string;
  enabled: boolean;
  created_at: string | null;
}

export interface LoopStageDto {
  // 后端用 #[serde(flatten)] 把 stage 拍平, 这里直接展开字段
  id: number;
  loop_id: number;
  order_index: number;
  name: string;
  description: string;
  todo_id: number;
  run_mode: string;
  skip_on_source_failed: boolean;
  min_rating: number | null;
  unrated_policy: string;
  enabled: boolean;
  created_at: string | null;
  // 冗余字段, 后端 JOIN 时一并返回, 避免前端再请求 todo 详情
  todo_title: string;
  todo_executor: string;
  todo_status: string;
}

export interface LoopHookDto {
  id: number;
  loop_id: number;
  hook_position: string;
  source_stage_id: number | null;
  target_todo_id: number;
  skip_if_missing: boolean;
  enabled: boolean;
  min_rating: number | null;
  unrated_policy: string;
  created_at: string | null;
}

export interface LoopExecutionDto {
  id: number;
  loop_id: number;
  trigger_id: number | null;
  trigger_type: string;
  trigger_meta: string;
  started_at: string;
  finished_at: string | null;
  status: string;
  total_stages: number;
  completed_stages: number;
  failed_stages: number;
}

export interface LoopStageExecutionDto {
  id: number;
  loop_execution_id: number;
  stage_id: number;
  todo_id: number;
  status: string;
  execution_record_id: number | null;
  error_message: string | null;
  started_at: string | null;
  finished_at: string | null;
}

export interface TodoSummaryForLoop {
  id: number;
  title: string;
  status: string;
  executor: string;
}

export interface LoopDetail {
  // 后端用 #[serde(flatten)] 把 LoopDto 拍平
  id: number;
  name: string;
  description: string;
  product: string;
  repo: string;
  branch: string;
  status: string;
  color: string;
  icon: string;
  created_at: string | null;
  updated_at: string | null;
  triggers: LoopTriggerDto[];
  stages: LoopStageDto[];
  hooks: LoopHookDto[];
  todo_map: Record<number, TodoSummaryForLoop>;
}

export interface LoopListItem {
  // 后端用 #[serde(flatten)] 把 LoopDto 拍平
  id: number;
  name: string;
  description: string;
  product: string;
  repo: string;
  branch: string;
  status: string;
  color: string;
  icon: string;
  created_at: string | null;
  updated_at: string | null;
  trigger_count: number;
  stage_count: number;
  last_execution_status: string;
  last_execution_at: string | null;
}

export interface LoopExecutionDetail {
  // 后端用 #[serde(flatten)] 把 execution 拍平
  id: number;
  loop_id: number;
  trigger_id: number | null;
  trigger_type: string;
  trigger_meta: string;
  started_at: string;
  finished_at: string | null;
  status: string;
  total_stages: number;
  completed_stages: number;
  failed_stages: number;
  stage_executions: LoopStageExecutionDto[];
  loop_name: string;
}

export interface LoopExecutionListResponse {
  items: LoopExecutionDto[];
  total: number;
  page: number;
  limit: number;
}

// ─── Request types ────────────────────────────────────────

export interface CreateLoopRequest {
  name: string;
  description?: string;
  product?: string;
  repo?: string;
  branch?: string;
  color?: string;
  icon?: string;
}

export interface UpdateLoopRequest {
  // 后端要求全量更新, 所有字段必填
  name: string;
  description: string;
  product: string;
  repo: string;
  branch: string;
  color: string;
  icon: string;
}

export interface CreateStageRequest {
  name: string;
  description?: string;
  /** 必须是 kind=expert 的 todo.id (后端强校验) */
  todo_id: number;
  run_mode?: LoopRunMode | string;
  skip_on_source_failed?: boolean;
  min_rating?: number | null;
  unrated_policy?: LoopUnratedPolicy | string;
  enabled?: boolean;
}

export interface UpdateStageRequest {
  // 后端要求全量更新
  name: string;
  description: string;
  todo_id: number;
  run_mode: string;
  skip_on_source_failed: boolean;
  min_rating: number | null;
  unrated_policy: string;
  enabled: boolean;
}

export interface ReorderStagesRequest {
  ordered_ids: number[];
}

export interface CreateTriggerRequest {
  trigger_type: LoopTriggerType | string;
  config?: string; // 默认 "{}"
  enabled?: boolean;
  priority?: number;
}

export interface UpdateTriggerRequest {
  trigger_type: LoopTriggerType | string;
  config: string;
  enabled: boolean;
  priority: number;
}

export interface CreateHookRequest {
  hook_position: LoopHookPosition | string;
  source_stage_id?: number | null;
  target_todo_id: number;
  skip_if_missing?: boolean;
  enabled?: boolean;
  min_rating?: number | null;
  unrated_policy?: LoopUnratedPolicy | string;
}

export interface UpdateHookRequest {
  hook_position: LoopHookPosition | string;
  source_stage_id: number | null;
  target_todo_id: number;
  skip_if_missing: boolean;
  enabled: boolean;
  min_rating: number | null;
  unrated_policy: LoopUnratedPolicy | string;
}

export interface UpdateLoopStatusRequest {
  status: LoopStatus | string;
}

export interface LoopExecutionListQuery {
  page?: number;
  limit?: number;
}

export interface LoopTriggerResponse {
  execution_id: number;
}
