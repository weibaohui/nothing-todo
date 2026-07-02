# Action Button Component — 可复用的一键执行组件

## 背景

在 ntd 的多个页面中，存在「选中一段文本 → 用 AI 优化 → 应用结果」的重复模式。例如：
- Todo 标题重写
- Prompt 优化
- 验收标准生成
- 内容摘要

目前每次实现这类功能都需要手动编写：创建 todo → 执行 → 等待结果 → 提取结论 → 应用/拒绝。逻辑高度重复，且缺乏统一的 UI 交互模式。

## 目标

封装一个**前后端配合的可复用组件**，在任意页面中一行代码即可接入「一键 AI 执行」能力。

### 核心交互流程

```
用户点击按钮
  → 弹出执行面板（Drawer/Modal）
  → 展示当前值（原标题/原 Prompt）
  → 用户点击「执行」
  → 后台创建临时 todo，将当前值作为 message 传入，启动执行
  → 前端轮询/WebSocket 等待完成
  → 提取执行结论，展示「原文 → 优化结果」对比
  → 用户选择「应用」或「拒绝」
    → 应用：调用 onApply 回调，由页面决定如何使用结果
    → 拒弃：关闭面板，不做任何修改
```

## 架构设计

### 后端：Action Execution API

新增两个 API 端点，复用现有 `executor_service` 执行体系。

#### 1. `POST /api/actions/execute` — 启动执行

**请求体：**
```json
{
  "todo_id": 123,              // 必填：使用哪个 todo 的 prompt 作为执行模板
  "message": "要优化的标题或内容", // 必填：传入的输入文本
  "params": {},                // 可选：额外模板参数（如 {{language}} 等）
  "executor": "claudecode"     // 可选：覆盖执行器类型
}
```

**响应体：**
```json
{
  "code": 0,
  "data": {
    "record_id": 456,          // 执行记录 ID，用于后续查询结果
    "task_id": "uuid-xxx"      // 任务 ID，用于 WebSocket 事件追踪
  }
}
```

**设计取舍：**
- 复用现有 `POST /api/execute` 的执行逻辑，但不直接暴露该接口给 Action Button 组件。
  原因：`/api/execute` 是通用执行入口，缺少「提取结论」的语义；新接口可以附加 action-specific 的后处理。
- `todo_id` 必填：由调用方指定使用哪个 todo 的 prompt 模板。不同 action type（标题重写、Prompt 优化）
  对应不同的 prompt 模板，这些模板作为普通 todo 存在于系统中，由管理员在 UI 中配置。
- 不新建 action_templates 表：复用 todo 表 + todo_templates 表，降低 schema 复杂度。

#### 2. `GET /api/actions/result/:record_id` — 查询执行结果

**响应体：**
```json
{
  "code": 0,
  "data": {
    "record_id": 456,
    "status": "success",        // "running" | "success" | "failed"
    "result": "优化后的标题文本", // 从执行输出中提取的结论（纯文本）
    "raw_output": "...",        // 完整的执行输出（可选，用于调试）
    "error": null               // 失败时的错误信息
  }
}
```

**结果提取逻辑：**
- 执行成功时，从 `execution_records.result` 字段读取。
  `result` 由 executor 的 completion handler 在执行结束时写入，通常是 AI 输出的最终结论。
- 执行中时返回 `status: "running"`，前端继续轮询。
- 执行失败时返回 `status: "failed"` + `error` 信息。

### 前端：ActionButton 组件

#### 组件接口

```typescript
interface ActionButtonProps {
  /** 使用哪个 todo 的 prompt 作为执行模板 */
  todoId: number;
  /** 要处理的输入文本 */
  input: string;
  /** 执行完成后「应用」的回调，参数为 AI 生成的结果文本 */
  onApply: (result: string) => void | Promise<void>;
  /** 按钮显示内容 */
  children?: React.ReactNode;
  /** 按钮类型（Ant Design） */
  buttonType?: 'primary' | 'default' | 'link' | 'text';
  /** 按钮图标 */
  icon?: React.ReactNode;
  /** 是否禁用 */
  disabled?: boolean;
  /** 面板标题（默认：智能执行） */
  panelTitle?: string;
  /** 面板描述（默认：将使用 AI 处理以下内容） */
  panelDescription?: string;
  /** 额外的模板参数 */
  params?: Record<string, string>;
  /** 覆盖执行器类型 */
  executor?: string;
  /** 面板宽度（默认 480） */
  panelWidth?: number;
}
```

#### 组件内部状态机

```
IDLE → EXECUTING → COMPLETED → (APPLIED | REJECTED)
         ↓
       FAILED
```

#### 组件行为

1. **IDLE 状态**：渲染一个 Button，点击后打开 Drawer（PC）/ Bottom Sheet（移动端）。
   面板内容：
   - 标题：`panelTitle`
   - 描述：`panelDescription`
   - 输入预览：展示 `input` 值（只读，灰色背景）
   - 底部按钮：「取消」+「执行」

2. **EXECUTING 状态**：面板切换为 loading 态。
   - 显示 Spin + "AI 正在处理中..."
   - 底部按钮禁用
   - 后端开始执行，前端每 2 秒轮询 `GET /api/actions/result/:record_id`
   - 同时监听 WebSocket 的 `Finished` 事件，收到后立即查询结果（减少轮询延迟）

3. **COMPLETED 状态**：面板展示对比结果。
   - 原文（input）→ 新文（result）的对比视图
   - 底部按钮：「拒绝」+「应用」

4. **FAILED 状态**：面板展示错误信息。
   - 错误原因
   - 底部按钮：「关闭」+「重试」

5. **APPLIED / REJECTED**：关闭面板，调用 `onApply(result)` 或不做任何事。

#### 使用示例

```tsx
// 在 Todo 标题编辑页
<ActionButton
  todoId={rewriteTitleTemplateId}
  input={currentTitle}
  onApply={(newTitle) => {
    setTitle(newTitle);
    saveTodo({ title: newTitle });
  }}
  panelTitle="重写标题"
  panelDescription="AI 将根据当前标题生成更优的版本"
>
  <EditOutlined /> 一键优化标题
</ActionButton>

// 在 Prompt 编辑页
<ActionButton
  todoId={optimizePromptTemplateId}
  input={currentPrompt}
  onApply={(newPrompt) => {
    setPrompt(newPrompt);
    saveTodo({ prompt: newPrompt });
  }}
  panelTitle="优化 Prompt"
  panelDescription="AI 将优化提示词的表达，使其更清晰有效"
  params={{ language: 'zh' }}
>
  <RocketOutlined /> 优化 Prompt
</ActionButton>
```

## 文件结构

```
backend/src/handlers/
  action.rs              # 新增：POST /api/actions/execute + GET /api/actions/result/:id

frontend/src/components/
  ActionButton/
    index.tsx            # 主组件
    ActionPanel.tsx      # 执行面板（Drawer/Modal 内容）
    ResultCompare.tsx    # 结果对比视图
    types.ts             # 类型定义
    hooks/
      useActionExecution.ts  # 执行状态管理 + 轮询逻辑
```

## 后端实现要点

### `action.rs` handler

```rust
// POST /api/actions/execute
pub async fn execute_action(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ExecuteActionRequest>,
) -> Result<ApiResponse<ExecuteActionResult>, AppError> {
    // 1. 查询 todo，验证存在
    // 2. 检查并发限制（复用 execute_handler 的逻辑）
    // 3. 替换 prompt 中的 {{message}} 占位符
    // 4. 调用 start_todo_execution 启动执行
    // 5. 返回 record_id + task_id
}

// GET /api/actions/result/:record_id
pub async fn get_action_result(
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
) -> Result<ApiResponse<ActionResultResponse>, AppError> {
    // 1. 查询 execution_record
    // 2. 根据 status 返回不同结构
    // 3. 成功时从 result 字段提取结论
}
```

### 路由注册

在 `handlers/mod.rs` 的 `execution_routes()` 中新增：
```rust
.route("/api/actions/execute", post(action::execute_action))
.route("/api/actions/result/{record_id}", get(action::get_action_result))
```

## 前端实现要点

### `useActionExecution` Hook

```typescript
function useActionExecution(todoId: number, message: string, params?: Record<string, string>) {
  // 状态：idle | executing | completed | failed
  // execute(): POST /api/actions/execute → 开始执行
  // pollResult(): 每 2 秒 GET /api/actions/result/:record_id
  // 通过 WebSocket Finished 事件触发即时查询
  // 返回：{ status, result, error, execute, retry, cancel }
}
```

### 移动端适配

- PC 端使用 `Drawer`（右侧滑出）
- 移动端使用 Ant Design 的 `Drawer placement="bottom"` 或自定义 Bottom Sheet
- 通过 `isMobile` prop 或 `useDevice()` hook 自动切换

## 配置与扩展

### 如何新增一种 Action

1. 在 ntd 中创建一个 todo，编写专用的 prompt 模板：
   ```
   你是一个标题优化专家。请根据以下标题生成 3 个更优的版本，要求：
   1. 保持原意
   2. 更简洁有力
   3. 适合 AI Todo 应用的场景

   原标题：{{message}}

   请直接输出最优版本，不要加解释。
   ```
2. 记住这个 todo 的 ID
3. 在前端使用 `<ActionButton todoId={该ID} .../>`

无需修改后端代码，无需建表，无需部署。所有 action 类型都是普通 todo。

## 验收标准

- [ ] 后端：`POST /api/actions/execute` 能正确创建并执行 todo
- [ ] 后端：`GET /api/actions/result/:record_id` 能正确返回执行状态和结果
- [ ] 前端：ActionButton 组件能在 PC 和移动端正常渲染
- [ ] 前端：执行过程中有 loading 状态，完成后展示对比结果
- [ ] 前端：应用/拒绝按钮正常工作
- [ ] 前端：组件可复用，在 TodoPage 和 TodoPostPage 中各接入一个实例
- [ ] 后端单元测试通过：`cd backend && cargo test`
- [ ] 前端 Playwright 验证通过
