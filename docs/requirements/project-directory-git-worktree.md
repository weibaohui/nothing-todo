# 项目目录 Git Worktree 功能需求

## 需求概述

在项目目录设置中增加两个属性：
1. **启用 Git Worktree**：勾选后，执行 Todo 时自动基于 main 分支创建独立的 worktree
2. **自动清理**：勾选后，Todo 执行完成（包括成功/失败）时自动删除该 worktree

同时，如果目录不是 Git 仓库，自动执行 `git init` 初始化。

---

## 背景与现状分析

### 现有实现

| 层级 | 表 | worktree 相关字段 | 当前行为 |
|------|-----|-----------------|---------|
| Todo | `todos` | `worktree_enabled: bool` | 仅给 Claude Code/Hermes 插入 `--worktree` 参数 |
| 项目目录 | `project_directories` | **无** | 不支持 |

现有 `apply_worktree_flag()` 函数（`executor_service.rs:123`）：
- 只对 `Claudecode` 和 `Hermes` 执行器生效
- 只是在命令行插入 `--worktree` 标志
- **不负责创建/清理 worktree**，依赖 Claude Code 自身管理

### 用户期望的新行为

由 ntd 程序（而非 Claude Code）完全托管 worktree 的生命周期：
1. 执行前：程序自动创建 worktree
2. 执行中：记录 worktree 路径到执行历史
3. 执行后：程序自动清理 worktree（如果勾选了自动清理）

---

## 功能详述

### 1. 数据库变更

#### 1.1 `project_directories` 表新增字段

```sql
ALTER TABLE project_directories ADD COLUMN git_worktree_enabled INTEGER DEFAULT 0;
ALTER TABLE project_directories ADD COLUMN auto_cleanup INTEGER DEFAULT 0;
```

Rust Entity 变更 (`backend/src/db/entity/project_directories.rs`)：
```rust
pub struct Model {
    // ... 现有字段 ...
    pub git_worktree_enabled: Option<bool>,  // 新增
    pub auto_cleanup: Option<bool>,           // 新增
}
```

#### 1.2 `execution_records` 表新增字段

```sql
ALTER TABLE execution_records ADD COLUMN worktree_path TEXT;
```

用于记录本次执行使用的 worktree 目录路径。

Rust Entity 变更 (`backend/src/db/entity/execution_records.rs`)：
```rust
pub struct Model {
    // ... 现有字段 ...
    pub worktree_path: Option<String>,  // 新增
}
```

### 2. 前端变更

#### 2.1 `ProjectDirectoriesPanel.tsx`

在项目目录列表的每一行或详情弹窗中，增加两个 Switch 复选框：
- "启用 Git Worktree" - 勾选后该目录下的 Todo 执行时使用 worktree
- "自动清理" - 勾选后执行完成自动删除 worktree（依赖启用 Git Worktree）

布局示意：
```
项目名称: my-project
路径: /path/to/my-project

[ ] 启用 Git Worktree    [ ] 自动清理
```

#### 2.2 类型定义更新

`frontend/src/utils/database/todos.ts` - `ProjectDirectory` 接口：
```typescript
export interface ProjectDirectory {
  id: number;
  path: string;
  name: string | null;
  created_at: string;
  updated_at: string;
  git_worktree_enabled?: boolean;  // 新增
  auto_cleanup?: boolean;          // 新增
}
```

#### 2.3 API 调用更新

`frontend/src/utils/database/todos.ts`:
- `createProjectDirectory(path, name)` → 增加可选参数 `gitWorktreeEnabled`, `autoCleanup`
- `updateProjectDirectory(id, name)` → 增加可选参数 `gitWorktreeEnabled`, `autoCleanup`

### 3. 后端变更

#### 3.1 API Handler 变更

`backend/src/handlers/project_directory.rs`：
- `CreateProjectDirectoryRequest` 增加 `git_worktree_enabled`, `auto_cleanup` 字段
- `UpdateProjectDirectoryRequest` 增加 `git_worktree_enabled`, `auto_cleanup` 字段
- 创建/更新时，如果 `git_worktree_enabled=true` 且目录不是 git 仓库，自动执行 `git init`

#### 3.2 Worktree 管理服务

新建 `backend/src/services/worktree.rs`，实现：

```rust
pub struct WorktreeService;

impl WorktreeService {
    /// 检查目录是否是 git 仓库，不是则初始化
    pub async fn ensure_git_repo(path: &str) -> Result<(), WorktreeError>;

    /// 基于 main 分支创建 worktree
    /// 返回 worktree 目录路径，格式: /path/to/project/.worktrees/<todo_id>-<timestamp>
    pub async fn create_worktree(project_path: &str, todo_id: i64) -> Result<String, WorktreeError>;

    /// 删除 worktree
    pub async fn cleanup_worktree(worktree_path: &str) -> Result<(), WorktreeError>;
}
```

Worktree 目录规范：
- 存储位置：`<project_path>/.worktrees/<todo_id>-<timestamp>/`
- 例如：`/path/to/project/.worktrees/123-0617123456/`
- `.worktrees` 目录加入 `.gitignore`

#### 3.3 执行服务集成

`backend/src/executor_service.rs`：

修改 `run_todo_execution` 流程：

```
1. 加载 todo
2. 获取 todo 对应的 workspace（项目目录路径）
3. 检查项目目录是否启用 git_worktree_enabled
4. 如果启用：
   a. 调用 WorktreeService::ensure_git_repo() 确保是 git 仓库
   b. 调用 WorktreeService::create_worktree() 创建 worktree
   c. 将 worktree_path 记录到 execution_record
   d. 将 worktree_path 作为执行目录（而非原始 workspace）
5. 执行任务（使用 worktree 目录）
6. 执行完成检查：
   a. 如果启用了 auto_cleanup，调用 WorktreeService::cleanup_worktree()
7. 保存 execution_record（包含 worktree_path）
```

#### 3.4 状态变更触发清理

如果用户在执行过程中手动取消任务（`Cancelled` 状态），也需要检查 auto_cleanup 并执行清理。

### 4. 执行记录展示

前端执行历史记录页面（`ExecutionRecordsPanel` 或类似组件）：
- 增加 "Worktree 路径" 列
- 显示该次执行使用的 worktree 目录地址

---

## 风险与边界条件

### 边界条件处理

| 场景 | 预期行为 |
|------|---------|
| 目录不是 git 仓库 | 自动 `git init`，继续执行 |
| `git init` 失败（如权限问题） | 返回错误，终止执行 |
| main 分支不存在 | 创建 main 分支或报错提示用户 |
| worktree 目录已存在（如同名 todo 重复执行） | 使用已有 worktree 或先清理再创建 |
| 执行过程中目录被删除 | 捕获错误，记录到执行历史 |
| auto_cleanup=true 但 worktree 已被手动删除 | 忽略，继续执行 |
| 目录在非 git 项目目录下 | 报错："请先初始化 git 仓库" |

### 并发处理

- 同一个项目目录可以同时运行多个 Todo
- 每个 Todo 有独立的 worktree 目录（通过 todo_id 区分）
- 不会相互干扰

---

## 数据流图

```
用户创建 Todo (workspace=项目目录)
        ↓
执行 Todo
        ↓
检查项目目录的 git_worktree_enabled
        ↓ (启用)
┌─→ 确保 git 仓库 (git init 如果需要)
│         ↓
├─→ 创建 worktree (.worktrees/<todo_id>-<timestamp>)
│         ↓
├─→ 执行任务（在 worktree 目录内）
│         ↓
├─→ 保存 execution_record (包含 worktree_path)
│         ↓
└─→ 检查 auto_cleanup
          ↓ (启用)
    清理 worktree
```

---

## 测试要点

### 单元测试

1. **WorktreeService**
   - `test_ensure_git_repo_existing_repo`
   - `test_ensure_git_repo_non_existing_repo_initializes`
   - `test_create_worktree_generates_correct_path`
   - `test_create_worktree_existing_fails_gracefully`
   - `test_cleanup_worktree_removes_directory`

2. **ProjectDirectory Handler**
   - `test_create_with_worktree_flags`
   - `test_update_worktree_flags`
   - `test_create_auto_inits_git_when_enabled`

3. **ExecutorService 集成**
   - `test_worktree_created_before_execution`
   - `test_worktree_cleaned_after_completion`
   - `test_worktree_cleaned_on_cancellation`

### 集成测试

1. 创建启用 worktree 的项目目录
2. 创建绑定到该目录的 Todo
3. 执行 Todo
4. 验证 worktree 被创建
5. 验证执行历史包含 worktree_path
6. 验证执行完成后 worktree 被清理（如果 auto_cleanup=true）
7. 验证执行完成后 worktree 保留（如果 auto_cleanup=false）

---

## 工作量评估

| 模块 | 工作内容 | 复杂度 |
|------|---------|--------|
| 数据库 | 新增字段 + migration | 低 |
| 前端 | 表单 + API 调用 | 低 |
| 后端 Handler | API 扩展 + git init | 中 |
| WorktreeService | 核心逻辑 | 中 |
| 执行服务集成 | 生命周期钩入 | 中 |
| 执行记录字段 | 新增字段 + 展示 | 低 |
| 测试 | 单元 + 集成 | 中 |

---

## 相关文件清单

### 需要修改的后端文件

- `backend/src/db/entity/project_directories.rs` - 新增字段
- `backend/src/db/entity/execution_records.rs` - 新增字段
- `backend/src/db/project_directory.rs` - CRUD 扩展
- `backend/src/db/migration.rs` 或 `backend/src/db/migrations.rs` - 迁移脚本
- `backend/src/handlers/project_directory.rs` - API 扩展
- `backend/src/services/worktree.rs` - **新建**，核心服务
- `backend/src/executor_service.rs` - 集成 worktree 生命周期
- `backend/src/models/mod.rs` - Request/Response 模型

### 需要修改的前端文件

- `frontend/src/utils/database/todos.ts` - API 类型
- `frontend/src/components/settings/ProjectDirectoriesPanel.tsx` - UI
- `frontend/src/types/todo.ts` - 类型定义（如需要）

### 需要修改的文档

- `docs/user-guide/settings/project-directories.md` - 用户文档
