//! Git Worktree 服务（issue #643）
//!
//! 在项目目录级托管 git worktree 的完整生命周期。
//! 由 ntd（而不是 Claude Code 自身）负责：
//!   1. 执行前：若目录不是 git 仓库则 `git init`；然后 `git worktree add` 一个 worktree
//!   2. 执行中：把 worktree 路径回写到 execution_record（仅记录，不影响子进程）
//!   3. 执行后：若该目录启用了 `auto_cleanup`，调用 `git worktree remove --force` 清理
//!
//! 设计取舍：
//! - 所有 git 命令都用 `std::process::Command` 直接 spawn 同步执行（不用 git2 crate）：
//!   1. ntd 已经把 git 当作外部依赖（auto init / status 都靠 `Command::new("git")`），
//!      引入 git2 会多一层 Rust ABI 维护成本；
//!   2. git CLI 的错误信息更可读，调试更直观；
//!   3. 这部分逻辑只在前置/收尾阶段跑一次，不在 hot path，开销可以接受。
//! - 所有同步 git 调用统一走 `run_git_with_timeout` 包装，避免在 lock / I/O hang 时
//!   阻塞调用方线程。超时后会主动 `kill` 子进程并返回 WorktreeError::GitTimeout。
//! - worktree 目录名格式：`<todo_id>-<unix_micros>`。`unix_micros` 选微秒而不是纳秒，
//!   避免出现同名 worktree 时仅相差几纳秒无法区分；选微秒而非秒是因为同一秒内
//!   可能并发触发多次 worktree 创建（例如同一 todo 重试 / scheduler 抖动）。
//! - `cleanup_worktree` 在目录已不存在或 `git worktree remove` 失败时**不报错**：
//!   用户手动删除或 git 元数据丢失时，让"清理"成为幂等 no-op 而非阻塞执行结果。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use thiserror::Error;
use tracing::{info, warn};
use wait_timeout::ChildExt;

/// 单次 git 命令的硬超时。30 秒覆盖「首次 init + 空 commit」最坏路径，
/// 远高于 `git rev-parse` / `worktree add` 等轻量子命令的常态耗时。
const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

/// 在项目目录下创建 worktree 的相对目录名（issue #643 规范要求）。
///
/// 选 `.worktrees` 而不是 `worktrees` 是因为以 `.` 开头会被常见工具识别为"本地临时目录"，
/// 减少误提交风险；同时 ntd 启动时也会确保该目录在 `.gitignore` 里。
pub const WORKTREE_ROOT_DIR: &str = ".worktrees";

#[derive(Debug, Error)]
pub enum WorktreeError {
    #[error("git is not installed or not in PATH: {0}")]
    GitUnavailable(String),
    #[error("project directory does not exist: {0}")]
    ProjectDirMissing(String),
    #[error("`git {cmd}` failed in {dir}: {stderr}")]
    GitCommandFailed {
        cmd: String,
        dir: String,
        stderr: String,
    },
    /// git 子命令在 `GIT_COMMAND_TIMEOUT` 内未结束。kill 子进程后向上抛，
    /// 由调用方决定回退到原 workspace 还是直接报失败。
    #[error("`git {cmd}` in {dir} timed out after {timeout:?}")]
    GitTimeout {
        cmd: String,
        dir: String,
        timeout: Duration,
    },
}

/// 单实例无状态服务。
///
/// 这里用 unit struct 而不是 free function 集合，原因是 issue 描述里要求
/// "由 ntd 程序托管 worktree 生命周期" —— 用一个具名类型让调用方更明确
/// 表达"这是 worktree 相关操作"，未来加 metrics/tracing 接入也好挂。
pub struct WorktreeService;

/// 给同步 git 命令加超时边界。
///
/// 之所以自己包一层而不直接 `cmd.output()`：
///   - git 在持有锁、远端 I/O hang 时 `output()` 会无限阻塞，
///     把调用方所在 tokio worker 也拖死；
///   - 超时后必须主动 `kill` 子进程，否则即便我们返回 Err 也会留下孤儿 git。
///
/// 实现思路：在线程里跑 `cmd.output()`，把结果通过 channel 传出；主线程用
/// `recv_timeout` 等待。超时分支用 `Child::from_pid` 不行（我们没保留句柄），
/// 所以这里改为 `cmd.spawn() + wait_timeout` 直接同步等待，超时分支 `kill` 子进程。
///
/// 入参 `cmd_label` 用于在超时/失败时把「这条命令是啥」打到日志/错误信息里，
/// 方便排查。`cwd_display` 仅作错误日志用，current_dir 仍由调用方设置到 `cmd` 上。
fn run_git_with_timeout(
    mut cmd: Command,
    cmd_label: &str,
    cwd_display: &str,
) -> Result<std::process::Output, WorktreeError> {
    // 把输出重定向到管道，便于超时分支独立 kill 进程；不在超时分支读 stdout/stderr，
    // 减少 pipe 关闭的潜在阻塞。
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| WorktreeError::GitUnavailable(e.to_string()))?;
    // `wait_timeout` 是同步调用：返回 `Ok(Some(status))` 表示子进程已完成，
    // `Ok(None)` 表示还在跑（需要 kill），`Err` 通常意味着 wait 系统调用失败。
    match child
        .wait_timeout(GIT_COMMAND_TIMEOUT)
        .map_err(|e| WorktreeError::GitUnavailable(e.to_string()))?
    {
        Some(_) => {
            // 子进程已结束；用 `wait_with_output` 等价语义收集 stdout/stderr。
            // std 没有暴露「已经 wait 完但还要读 pipe」的 API，所以这里退化为
            // 重新调用 wait_with_output：对于正常结束的子进程，第二次 wait
            // 立刻返回已缓存的 ExitStatus，pipe 数据仍然可读。
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut s) = child.stdout.take() {
                use std::io::Read;
                let _ = s.read_to_end(&mut stdout);
            }
            if let Some(mut s) = child.stderr.take() {
                use std::io::Read;
                let _ = s.read_to_end(&mut stderr);
            }
            let status = child.wait().map_err(|e| {
                WorktreeError::GitUnavailable(e.to_string())
            })?;
            Ok(std::process::Output {
                status,
                stdout,
                stderr,
            })
        }
        None => {
            // 超时：先 kill 再 wait，避免僵尸进程
            warn!(
                cmd = cmd_label,
                dir = cwd_display,
                "git command exceeded timeout, killing child"
            );
            let _ = child.kill();
            let _ = child.wait();
            Err(WorktreeError::GitTimeout {
                cmd: cmd_label.to_string(),
                dir: cwd_display.to_string(),
                timeout: GIT_COMMAND_TIMEOUT,
            })
        }
    }
}

impl WorktreeService {
    pub fn new() -> Self {
        Self
    }

    /// 确保 `project_path` 是一个 git 仓库，不是则自动 `git init`。
    ///
    /// 返回 `Ok(())` 表示目录已是 git 仓库（无论是本来就存在还是刚 init）。
    /// 返回 `Err` 时仅在三种场景：
    ///   1. `project_path` 不存在
    ///   2. `git` 命令无法 spawn（PATH 里找不到）
    ///   3. `git init` / `git rev-parse` 子命令退出码非 0
    pub fn ensure_git_repo(&self, project_path: &str) -> Result<(), WorktreeError> {
        let p = Path::new(project_path);
        if !p.exists() {
            return Err(WorktreeError::ProjectDirMissing(project_path.to_string()));
        }

        // 用 `git rev-parse --git-dir` 探测：返回成功即表示已是 git 仓库，
        // 比检查 `.git` 子目录更稳（worktree 自身的 `.git` 是文件不是目录）。
        // 探测本身走超时路径：失败=不是仓库，Ok 但退出非零=同义。
        let mut probe_cmd = Command::new("git");
        probe_cmd
            .arg("rev-parse")
            .arg("--git-dir")
            .current_dir(p);
        match run_git_with_timeout(probe_cmd, "rev-parse --git-dir", project_path) {
            Ok(out) if out.status.success() => return Ok(()),
            Ok(_) => {
                // 不是仓库，下一步执行 init
                info!(project = %project_path, "initializing empty git repository");
            }
            Err(e) => {
                // 超时或 spawn 失败都按「不可用」处理，让外层走 fallback
                return Err(e);
            }
        }

        // init 主路径走超时包装；fallback 同样。两条路径都失败时把第二次的错误向上抛。
        let mut init_cmd = Command::new("git");
        init_cmd.arg("init").arg("-b").arg("main").current_dir(p);
        let init_out = run_git_with_timeout(init_cmd, "init -b main", project_path)?;
        if init_out.status.success() {
            return Ok(());
        }

        // 兜底：某些旧版 git 不支持 `-b main`，再用默认 init 重试
        let mut fallback_cmd = Command::new("git");
        fallback_cmd.arg("init").current_dir(p);
        let fallback = run_git_with_timeout(fallback_cmd, "init", project_path)?;
        if !fallback.status.success() {
            // 兜底也失败，错误信息通常来自 stderr；这里只能粗略标记，由调用方日志定位
            return Err(WorktreeError::GitCommandFailed {
                cmd: "init".into(),
                dir: project_path.to_string(),
                stderr: "git init failed after fallback".into(),
            });
        }
        Ok(())
    }

    /// 基于 `<project>/.worktrees/<todo_id>-<unix_micros>/` 下创建 worktree。
    ///
    /// 如果当前分支还不存在（仓库刚 init），则先建一个空 commit 避免
    /// `git worktree add` 报 "fatal: invalid reference"。
    ///
    /// 返回值是 worktree 目录的**绝对路径**，可直接作为 `Command::current_dir` 使用。
    pub fn create_worktree(
        &self,
        project_path: &str,
        todo_id: i64,
    ) -> Result<String, WorktreeError> {
        // 入口先做 git 仓库检查（包含自动 init），保证下面的 worktree add 不会
        // 在非 git 目录上失败；这一步在并发首次执行时是幂等的。
        self.ensure_git_repo(project_path)?;

        // 探测当前分支是否存在提交。空仓库 init 后没有 HEAD，需要先做一次空 commit
        // 才能 `git worktree add`；否则 git 会报 "invalid reference"。
        // 这里用 HEAD 而不是硬编码 "main"——很多环境下默认分支是 master，
        // 硬编码 main 会导致老仓库 worktree 创建失败。
        if !self.has_any_commit(project_path)? {
            self.ensure_empty_commit(project_path)?;
        }

        // 一次性清理旧版 ISO 8601 格式的遗留 worktree 目录。
        // 旧版本（年份作为时间戳的版本）会留下 `<todo_id>-2026-06-18T08:30:00.000Z` 这种
        // 目录命名，本版本永远不会命中；放任不管会持续累积孤儿目录与 dangling 分支。
        // 只清理当前 todo_id 下的目录，避免误删其他 todo 的活跃 worktree。
        self.prune_legacy_worktrees(project_path, todo_id);

        // 基于当前分支的 HEAD 创建 worktree，不再硬编码 "main"。
        // 当前分支名由 `current_branch` 探测得到，兼容 main/master/自定义分支。
        let base = self.current_branch(project_path)?;

        // 碰撞重试：同一微秒内的并发 / NTP 跳变 / 重试 race 仍可能撞到相同 timestamp，
        // 因此在 `git worktree add` 返回 "already exists" 时换一个新 timestamp 再试。
        // 最多 5 次；超出后让上层 fallback 到原始 workspace，不阻塞执行。
        const MAX_ATTEMPTS: u32 = 5;
        let mut last_err: Option<WorktreeError> = None;
        for attempt in 1..=MAX_ATTEMPTS {
            let (branch_name, worktree_dir) = Self::mint_worktree_identity(project_path, todo_id);

            if worktree_dir.exists() {
                // 目录先于分支存在：直接换下一个 timestamp，不调 git。
                warn!(
                    worktree = %worktree_dir.display(),
                    todo_id = todo_id,
                    attempt,
                    "worktree directory collision, retrying with new timestamp"
                );
                continue;
            }

            let mut add_cmd = Command::new("git");
            add_cmd
                .arg("worktree")
                .arg("add")
                .arg("-b")
                .arg(&branch_name)
                .arg(&worktree_dir)
                .arg(&base)
                .current_dir(project_path);
            let out = run_git_with_timeout(add_cmd, "worktree add", project_path)?;
            if out.status.success() {
                info!(
                    worktree = %worktree_dir.display(),
                    base = %base,
                    attempt,
                    "created git worktree for todo execution"
                );
                return Ok(worktree_dir.to_string_lossy().into_owned());
            }

            // 仅在「分支或目录已存在」时重试，其他错误直接返回。
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            if stderr.contains("already exists") {
                warn!(
                    worktree = %worktree_dir.display(),
                    attempt,
                    stderr = %stderr,
                    "git worktree add collision, retrying"
                );
                last_err = Some(WorktreeError::GitCommandFailed {
                    cmd: "worktree add".into(),
                    dir: project_path.to_string(),
                    stderr,
                });
                continue;
            }
            return Err(WorktreeError::GitCommandFailed {
                cmd: "worktree add".into(),
                dir: project_path.to_string(),
                stderr,
            });
        }
        Err(last_err.unwrap_or_else(|| WorktreeError::GitCommandFailed {
            cmd: "worktree add".into(),
            dir: project_path.to_string(),
            stderr: format!(
                "exceeded {} attempts on timestamp collision",
                MAX_ATTEMPTS
            ),
        }))
    }

    /// 生成 worktree 的「目录 + 分支」命名对，确保两者共享同一 timestamp 后缀。
    ///
    /// 抽出来是为了让目录名模板 `<todo_id>-<timestamp>` 和分支名模板
    /// `wt-<todo_id>-<timestamp>` 在源码上锚定在同一处；任何对命名格式的修改
    /// 都只改这一处，避免 cleanup 阶段按字符串拼接出来的分支名找不到对应目录。
    fn mint_worktree_identity(project_path: &str, todo_id: i64) -> (String, PathBuf) {
        let timestamp = Self::unique_timestamp();
        // 目录名 = 分支名去掉 "wt-" 前缀，保证两者尾部 `<todo_id>-<timestamp>` 一致。
        let identity = format!("{}-{}", todo_id, timestamp);
        let branch_name = format!("wt-{}", identity);
        let worktree_dir = PathBuf::from(project_path)
            .join(WORKTREE_ROOT_DIR)
            .join(&identity);
        (branch_name, worktree_dir)
    }

    /// 清理旧版 ISO 8601 格式的遗留 worktree 目录（如 `42-2026-06-18T08:30:00.000Z`）。
    ///
    /// 识别规则：目录名以 `<todo_id>-` 开头、且含 `T` 分隔符（ISO 8601 标志）。
    /// 命中后用 `git worktree remove --force` 优雅清理，失败则 fallback `rm -rf`，
    /// 保证 `git worktree prune` 不再留下 dangling 元数据。
    /// 只清理当前 todo_id 的目录，避免误删其他 todo 的活跃 worktree。
    fn prune_legacy_worktrees(&self, project_path: &str, todo_id: i64) {
        let root = Path::new(project_path).join(WORKTREE_ROOT_DIR);
        let entries = match std::fs::read_dir(&root) {
            Ok(e) => e,
            Err(_) => return, // 目录不存在无需清理
        };
        let prefix = format!("{}-", todo_id);
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // 仅清理以 `<todo_id>-` 开头、含 `T`（ISO 8601 分隔符）的旧目录
            if !name.starts_with(&prefix) || !name.contains('T') {
                continue;
            }
            let legacy = entry.path();
            // 先尝试 git worktree remove；失败再 rm -rf，与 cleanup_worktree 行为一致。
            let mut rm_cmd = Command::new("git");
            rm_cmd
                .arg("worktree")
                .arg("remove")
                .arg("--force")
                .arg(&legacy)
                .current_dir(project_path);
            let removed = match run_git_with_timeout(rm_cmd, "worktree remove --force (legacy)", project_path) {
                Ok(o) => o.status.success(),
                Err(_) => false,
            };
            if !removed {
                let _ = std::fs::remove_dir_all(&legacy);
            }
            // 回收 git 内部的 worktree 元数据
            let mut prune_cmd = Command::new("git");
            prune_cmd
                .arg("worktree")
                .arg("prune")
                .current_dir(project_path);
            let _ = run_git_with_timeout(prune_cmd, "worktree prune", project_path);
            info!(legacy = %legacy.display(), "pruned legacy ISO-format worktree");
        }
    }

    /// 清理 worktree。已不存在/已被手动删除/git 元数据丢失时一律记 warn 返回 Ok(())，
    /// 让 cleanup 步骤成为幂等 no-op。
    pub fn cleanup_worktree(&self, worktree_path: &str) -> Result<(), WorktreeError> {
        let path = Path::new(worktree_path);
        if !path.exists() {
            warn!(worktree = %worktree_path, "worktree path already gone, skip cleanup");
            return Ok(());
        }

        // 找 worktree 所属的主仓库（git 自身的 .git 文件记录了 gitdir 指向主仓的 worktrees/）
        let main_repo = match self.find_main_repo(worktree_path) {
            Ok(p) => p,
            Err(e) => {
                warn!(worktree = %worktree_path, error = %e, "cannot locate main repo, removing directory directly");
                let _ = std::fs::remove_dir_all(path);
                return Ok(());
            }
        };

        let mut rm_cmd = Command::new("git");
        rm_cmd
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(path)
            .current_dir(&main_repo);
        let out = run_git_with_timeout(rm_cmd, "worktree remove --force", &main_repo.to_string_lossy());
        match out {
            Ok(o) if o.status.success() => {
                info!(worktree = %worktree_path, "cleaned up git worktree");
                Ok(())
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
                warn!(worktree = %worktree_path, stderr = %stderr, "git worktree remove failed, falling back to rm -rf");
                let _ = std::fs::remove_dir_all(path);
                Ok(())
            }
            Err(e) => {
                warn!(worktree = %worktree_path, error = %e, "failed to spawn git worktree remove, falling back to rm -rf");
                let _ = std::fs::remove_dir_all(path);
                Ok(())
            }
        }
    }

    /// 获取微秒级唯一时间戳，用作 worktree 目录名和分支名。
    ///
    /// 选微秒而不是纳诺秒是因为：1）git 分支名不允许 `:` 和 `.`，纯数字最安全；
    /// 2）微秒足够区分同一毫秒内的多次调用，且数值不会过长。
    /// Unix epoch 以来的微秒数在可预见的未来都不会回环。
    fn unique_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_micros() as i64
    }

    /// worktree 目录的绝对路径（不含创建动作），便于单测与日志展示。
    pub fn worktree_path(&self, project_path: &str, todo_id: i64, timestamp: i64) -> PathBuf {
        // 目录名与分支名保持一致，均为 `{todo_id}-{timestamp}`
        PathBuf::from(project_path)
            .join(WORKTREE_ROOT_DIR)
            .join(format!("{}-{}", todo_id, timestamp))
    }

    /// 探测仓库是否有任意 commit（HEAD 是否解析得到）。
    /// 取代硬编码 "main" 的存在性检查——空仓库 init 后任何分支都没有 commit。
    fn has_any_commit(&self, project_path: &str) -> Result<bool, WorktreeError> {
        let mut cmd = Command::new("git");
        cmd.arg("rev-parse")
            .arg("--verify")
            .arg("HEAD")
            .current_dir(project_path);
        let out = run_git_with_timeout(cmd, "rev-parse --verify HEAD", project_path)?;
        Ok(out.status.success())
    }

    /// 获取当前分支名（空仓库 fallback 到默认 "main"）。
    /// 优先级：`rev-parse --abbrev-ref HEAD` → 当用户处于 detached HEAD 时退到 init.defaultBranch。
    fn current_branch(&self, project_path: &str) -> Result<String, WorktreeError> {
        let mut cmd = Command::new("git");
        cmd.arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .current_dir(project_path);
        let probe = run_git_with_timeout(cmd, "rev-parse --abbrev-ref HEAD", project_path)?;
        if probe.status.success() {
            let name = String::from_utf8_lossy(&probe.stdout).trim().to_string();
            // detached HEAD 时 git 会输出 "HEAD"，不是真正的分支名
            if !name.is_empty() && name != "HEAD" {
                return Ok(name);
            }
        }
        // 兜底：空仓库时没有 HEAD，但 `git init -b main` 仍会创建 main 分支引用。
        // 即便没有 commit，后续 `worktree add ... main` 在空仓库也会失败——
        // 这正是 `ensure_empty_commit` 介入的时机。这里只兜底分支名探测。
        Ok("main".to_string())
    }

    /// 在空仓库的 main 分支上建一个空 commit，让后续 `git worktree add main` 不报 invalid reference。
    fn ensure_empty_commit(&self, project_path: &str) -> Result<(), WorktreeError> {
        // 注意：必须用环境变量注入 author/committer 身份，而不是 `git config --local`。
        // 原因：某些精简 git 镜像（CI/容器）下 `safe.directory` 限制会让 `git config --local`
        // 静默失败，导致 commit 时 "unable to auto-detect email address" 报错。
        // 环境变量绕过配置层，是 git 官方推荐的"一次性提交"做法。
        let mut cmd = Command::new("git");
        cmd.args(["commit", "--allow-empty", "-m", "ntd: initial worktree base"])
            .current_dir(project_path)
            .env("GIT_AUTHOR_NAME", "ntd")
            .env("GIT_AUTHOR_EMAIL", "ntd@localhost")
            .env("GIT_COMMITTER_NAME", "ntd")
            .env("GIT_COMMITTER_EMAIL", "ntd@localhost");
        let commit = run_git_with_timeout(cmd, "commit --allow-empty", project_path)?;
        if !commit.status.success() {
            let stderr = String::from_utf8_lossy(&commit.stderr).into_owned();
            return Err(WorktreeError::GitCommandFailed {
                cmd: "commit --allow-empty".into(),
                dir: project_path.to_string(),
                stderr,
            });
        }
        Ok(())
    }

    /// 从 worktree 内部读 `.git` 文件，找到主仓库目录。
    fn find_main_repo(&self, worktree_path: &str) -> Result<PathBuf, WorktreeError> {
        let dot_git = Path::new(worktree_path).join(".git");
        let content = std::fs::read_to_string(&dot_git).map_err(|e| {
            WorktreeError::GitCommandFailed {
                cmd: "read .git".into(),
                dir: dot_git.to_string_lossy().into_owned(),
                stderr: e.to_string(),
            }
        })?;
        // .git 文件内容形如 `gitdir: /path/to/main/.git/worktrees/<name>`
        let gitdir = content
            .trim_start_matches("gitdir:")
            .trim()
            .to_string();
        // 取出 `/path/to/main/.git/worktrees/<name>` 中的 `/path/to/main` 段。
        let p = PathBuf::from(&gitdir);
        let ancestors: Vec<_> = p.ancestors().collect();
        // ancestors 顺序: worktree_name -> worktrees -> .git -> main_repo
        if ancestors.len() >= 4 {
            Ok(ancestors[3].to_path_buf())
        } else {
            Err(WorktreeError::GitCommandFailed {
                cmd: "parse .git".into(),
                dir: worktree_path.to_string(),
                stderr: format!("unexpected gitdir format: {}", gitdir),
            })
        }
    }
}

impl Default for WorktreeService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    /// 创建一个用 git init 过的临时目录并返回路径。
    /// 部分测试用例的"前置 init"需要在用例里显式调用 WorktreeService::ensure_git_repo，
    /// 这里只给一个直接走 CLI 的小 helper，避免用例代码被 git 命令细节淹没。
    fn init_temp_repo() -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let status = StdCommand::new("git")
            .arg("init")
            .current_dir(dir.path())
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");
        dir
    }

    /// 已存在仓库时 ensure_git_repo 是 no-op。
    #[test]
    fn test_ensure_git_repo_existing_repo_is_noop() {
        let dir = init_temp_repo();
        let svc = WorktreeService::new();
        svc.ensure_git_repo(dir.path().to_str().unwrap())
            .expect("existing repo should not error");
    }

    /// 目录不存在时返回 ProjectDirMissing。
    #[test]
    fn test_ensure_git_repo_missing_dir_errors() {
        let svc = WorktreeService::new();
        let res = svc.ensure_git_repo("/this/path/should/not/exist/ntd-test-643");
        assert!(matches!(res, Err(WorktreeError::ProjectDirMissing(_))));
    }

    /// 非 git 目录会自动 init。
    #[test]
    fn test_ensure_git_repo_non_existing_repo_initializes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let svc = WorktreeService::new();
        svc.ensure_git_repo(dir.path().to_str().unwrap())
            .expect("auto init should succeed");
        // init 之后再调一次应该依然 ok（命中"已是仓库"分支）
        svc.ensure_git_repo(dir.path().to_str().unwrap())
            .expect("re-call should be noop");
    }

    /// worktree_path 不依赖文件系统状态，只把 todo_id 拼进路径里。
    #[test]
    fn test_worktree_path_format() {
        let svc = WorktreeService::new();
        let ts = 1_234_567_890_000_i64; // 微秒时间戳
        let p = svc.worktree_path("/tmp/proj", 42, ts);
        let s = p.to_string_lossy();
        assert_eq!(s, "/tmp/proj/.worktrees/42-1234567890000");
    }

    /// 完整 create + cleanup 流程，验证 worktree 真的被 git 管起来。
    /// 跳过的条件：本机没装 git。CI 上没有 git 也能编过（test 不依赖 git 可用性）。
    #[test]
    fn test_create_and_cleanup_worktree_full_cycle() {
        if StdCommand::new("git").arg("--version").output().is_err() {
            // 没装 git 就跳过，避免在精简镜像里挂掉
            return;
        }
        let dir = init_temp_repo();
        // 给 main 一个空 commit，否则 worktree add 报 invalid reference
        // 必须设 author env，否则容器/CI 上 git 报 "unable to auto-detect email"。
        let status = StdCommand::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .env("GIT_AUTHOR_NAME", "ntd")
            .env("GIT_AUTHOR_EMAIL", "ntd@localhost")
            .env("GIT_COMMITTER_NAME", "ntd")
            .env("GIT_COMMITTER_EMAIL", "ntd@localhost")
            .current_dir(dir.path())
            .status()
            .expect("git commit");
        assert!(status.success(), "initial empty commit should succeed");

        let svc = WorktreeService::new();
        let wt = svc
            .create_worktree(dir.path().to_str().unwrap(), 1)
            .expect("create worktree");
        let wt_path = PathBuf::from(&wt);
        assert!(wt_path.exists(), "worktree dir should exist after create");
        // .worktrees 子目录在主仓下应当存在
        let wt_root = dir.path().join(WORKTREE_ROOT_DIR);
        assert!(wt_root.exists(), ".worktrees root should be created");

        svc.cleanup_worktree(&wt).expect("cleanup should be ok");
        // cleanup 后 worktree 目录应该消失（git worktree remove 会删目录）
        assert!(!wt_path.exists(), "worktree dir should be removed after cleanup");
    }

    /// cleanup 在目录已经不存在时返回 Ok(()), 验证幂等性。
    #[test]
    fn test_cleanup_worktree_missing_path_is_idempotent() {
        let svc = WorktreeService::new();
        svc.cleanup_worktree("/tmp/ntd-643-nonexistent-path")
            .expect("missing path cleanup should not error");
    }

    /// 仓库里没有 main 分支时，create_worktree 会自动建一个空 commit 让 main 可用。
    /// 验证空仓库 + worktree 也能工作（这是首次启用 worktree 的真实场景）。
    #[test]
    fn test_create_worktree_on_fresh_empty_repo() {
        if StdCommand::new("git").arg("--version").output().is_err() {
            return;
        }
        let dir = init_temp_repo();
        let svc = WorktreeService::new();
        let wt = svc
            .create_worktree(dir.path().to_str().unwrap(), 7)
            .expect("create worktree on fresh repo should succeed");
        assert!(PathBuf::from(&wt).exists());
        // 清理避免污染 /tmp（tempdir drop 会兜底删主目录，但 worktree 在子目录）
        let _ = fs::remove_dir_all(dir.path().join(WORKTREE_ROOT_DIR));
    }

    /// 验证旧版 ISO 8601 格式目录会被 `prune_legacy_worktrees` 清理。
    /// 模拟升级前留下的 `<todo_id>-<ISO>` 目录，调用 prune 后必须消失；
    /// 同时确认其他 todo 的目录不会被误删。
    #[test]
    fn test_prune_legacy_worktrees_removes_iso_format() {
        if StdCommand::new("git").arg("--version").output().is_err() {
            return;
        }
        let dir = init_temp_repo();
        // 给主仓库一个空 commit（worktree add 需要 HEAD 可解析）
        let status = StdCommand::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .env("GIT_AUTHOR_NAME", "ntd")
            .env("GIT_AUTHOR_EMAIL", "ntd@localhost")
            .env("GIT_COMMITTER_NAME", "ntd")
            .env("GIT_COMMITTER_EMAIL", "ntd@localhost")
            .current_dir(dir.path())
            .status()
            .expect("git commit");
        assert!(status.success());

        // 在 .worktrees 下放两个遗留目录：todo_id=5 的 ISO 格式 + todo_id=8 的 ISO 格式
        let root = dir.path().join(WORKTREE_ROOT_DIR);
        fs::create_dir_all(&root).unwrap();
        let legacy_self = root.join("5-2026-06-18T08:30:00.000Z");
        let legacy_other = root.join("8-2026-06-18T08:30:00.000Z");
        fs::create_dir_all(&legacy_self).unwrap();
        fs::create_dir_all(&legacy_other).unwrap();
        assert!(legacy_self.exists() && legacy_other.exists());

        let svc = WorktreeService::new();
        svc.prune_legacy_worktrees(dir.path().to_str().unwrap(), 5);

        // 只清 todo_id=5 的遗留目录，todo_id=8 的保留
        assert!(!legacy_self.exists(), "todo_id=5 legacy dir should be pruned");
        assert!(legacy_other.exists(), "todo_id=8 legacy dir should NOT be pruned");

        // 清理剩余目录避免污染
        let _ = fs::remove_dir_all(&root);
    }

    /// 验证 `mint_worktree_identity` 的目录+分支命名对保持一致。
    /// 两者必须共享同一 `<todo_id>-<timestamp>` 尾巴，否则 cleanup 时按
    /// 字符串推导出来的分支名找不到对应目录，会留下 dangling 元数据。
    #[test]
    fn test_mint_worktree_identity_keeps_branch_and_dir_in_sync() {
        let (branch, dir) = WorktreeService::mint_worktree_identity("/tmp/proj", 42);
        // 分支名形如 wt-42-1718695800000000，目录名形如 .../42-1718695800000000
        assert!(branch.starts_with("wt-42-"));
        let dir_name = dir.file_name().unwrap().to_string_lossy().into_owned();
        // 目录名末段必须 = 分支名去掉 "wt-" 前缀
        assert_eq!(dir_name, branch.trim_start_matches("wt-"));
    }
}
