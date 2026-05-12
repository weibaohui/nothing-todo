use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};

use crate::db::Database;
use crate::db::entity::todo_templates;
use crate::models::TodoTemplate;

impl Database {
    pub async fn get_templates(&self) -> Result<Vec<TodoTemplate>, sea_orm::DbErr> {
        let models = todo_templates::Entity::find()
            .order_by_asc(todo_templates::Column::SortOrder)
            .order_by_asc(todo_templates::Column::Id)
            .all(&self.conn)
            .await?;
        Ok(models
            .into_iter()
            .map(|m| TodoTemplate {
                id: m.id,
                title: m.title,
                prompt: m.prompt,
                category: m.category,
                sort_order: m.sort_order.unwrap_or(0),
                created_at: m.created_at,
                updated_at: m.updated_at,
            })
            .collect())
    }

    pub async fn get_templates_by_category(&self, category: &str) -> Result<Vec<TodoTemplate>, sea_orm::DbErr> {
        let models = todo_templates::Entity::find()
            .filter(todo_templates::Column::Category.eq(category.to_string()))
            .order_by_asc(todo_templates::Column::SortOrder)
            .order_by_asc(todo_templates::Column::Id)
            .all(&self.conn)
            .await?;
        Ok(models
            .into_iter()
            .map(|m| TodoTemplate {
                id: m.id,
                title: m.title,
                prompt: m.prompt,
                category: m.category,
                sort_order: m.sort_order.unwrap_or(0),
                created_at: m.created_at,
                updated_at: m.updated_at,
            })
            .collect())
    }

    pub async fn create_template(
        &self,
        title: &str,
        prompt: Option<&str>,
        category: &str,
        sort_order: Option<i32>,
    ) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = todo_templates::ActiveModel {
            title: ActiveValue::Set(title.to_string()),
            prompt: ActiveValue::Set(prompt.map(String::from)),
            category: ActiveValue::Set(category.to_string()),
            sort_order: ActiveValue::Set(sort_order),
            created_at: ActiveValue::Set(Some(now.clone())),
            updated_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        };
        let inserted = am.insert(&self.conn).await?;
        Ok(inserted.id)
    }

    pub async fn update_template(
        &self,
        id: i64,
        title: &str,
        prompt: Option<&str>,
        category: &str,
        sort_order: Option<i32>,
    ) -> Result<(), sea_orm::DbErr> {
        let model = todo_templates::Entity::find_by_id(id)
            .one(&self.conn)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Template not found".to_string()))?;

        let mut am: todo_templates::ActiveModel = model.into();
        am.title = ActiveValue::Set(title.to_string());
        am.prompt = ActiveValue::Set(prompt.map(String::from));
        am.category = ActiveValue::Set(category.to_string());
        am.sort_order = ActiveValue::Set(sort_order);
        am.updated_at = ActiveValue::Set(Some(crate::models::utc_timestamp()));
        am.update(&self.conn).await?;
        Ok(())
    }

    pub async fn delete_template(&self, id: i64) -> Result<(), sea_orm::DbErr> {
        todo_templates::Entity::delete_by_id(id).exec(&self.conn).await?;
        Ok(())
    }

    pub async fn seed_default_templates(&self) -> Result<(), sea_orm::DbErr> {
        // Check if templates already exist
        let existing = todo_templates::Entity::find()
            .one(&self.conn)
            .await?;
        if existing.is_some() {
            return Ok(());
        }

        let default_templates = vec![
            // 开发类
            ("代码审查", Some("审查以下代码，检查潜在 bug 和改进点：\n\n```\n[粘贴代码]\n```"), "开发", Some(1)),
            ("Bug 修复", Some("定位并修复以下问题：\n\n[描述问题]"), "开发", Some(2)),
            ("功能开发", Some("实现以下功能需求：\n\n[描述需求]"), "开发", Some(3)),
            ("代码重构", Some("重构以下代码，提升可读性和性能：\n\n```\n[粘贴代码]\n```"), "开发", Some(4)),
            ("性能优化", Some("分析并优化以下代码的性能：\n\n```\n[粘贴代码]\n```"), "开发", Some(5)),
            // 安全类
            ("安全审计", Some("检查以下代码的安全漏洞：\n\n```\n[粘贴代码]\n```"), "安全", Some(6)),
            // 测试类
            ("单元测试", Some("为以下代码编写单元测试：\n\n```\n[粘贴代码]\n```"), "测试", Some(7)),
            // 文档类
            ("文档撰写", Some("为以下功能编写文档：\n\n[描述功能]"), "文档", Some(8)),
            // 需求类
            ("需求分析", Some("分析以下需求并输出技术方案：\n\n[描述需求]"), "需求", Some(9)),
            // 学习类
            ("代码解释", Some("解释以下代码的功能和实现原理：\n\n```\n[粘贴代码]\n```"), "学习", Some(10)),
            // Codex 自动化类
            ("Bug 扫描与修复", Some("扫描最近的提交（自上次运行以来，或过去 24 小时内），查找可能的 bug 并提出最小修复方案。\n\n依据规则：\n- 只使用仓库中的具体证据（提交 SHA、PR、文件路径、diff、失败的测试、CI 信号）\n- 不要臆造 bug；如果证据不足，请说明并跳过\n- 优先选择最小且安全的修复；避免重构和无关清理"), "Codex", Some(11)),
            ("发布说明草稿", Some("根据已合并的 PR 起草每周发布说明（如有链接请附上）。\n\n范围与依据：\n- 严格限定在该仓库本周的历史记录内；不要添加超出数据支持的额外章节\n- 使用 PR 编号/标题；除非仓库中的 PR 描述、测试或指标支持，否则避免对影响作出结论"), "Codex", Some(12)),
            ("站会 Git 活动总结", Some("为站会总结昨天的 git 活动。\n\n依据规则：\n- 陈述需锚定到提交/PR/文件；不要臆测意图或未来工作\n- 保持便于快速浏览，适合团队同步"), "Codex", Some(13)),
            ("CI 失败总结与建议", Some("总结最近一个 CI 窗口中的 CI 失败和不稳定测试；提出首要修复建议。\n\n依据规则：\n- 尽可能引用具体作业、测试、错误信息或日志片段\n- 避免过度自信地断言根因；区分\"已观察到\"与\"疑似\""), "Codex", Some(14)),
            ("经典小游戏实现", Some("创建一个范围尽可能小的经典小游戏。\n\n约束：\n- 除非必要，否则不要添加额外功能、样式系统、内容或新的依赖\n- 复用现有仓库的工具和模式"), "Codex", Some(15)),
            ("性能回归对比", Some("将最近的更改与基准测试或追踪结果进行比较，并尽早标记回归。\n\n依据规则：\n- 以可测量的信号（基准测试、追踪、耗时、火焰图）为依据\n- 如果没有测量数据，请写明\"未找到测量数据\"，不要猜测"), "Codex", Some(16)),
            ("依赖版本漂移检测", Some("检测依赖项和 SDK 的版本漂移，并提出最小对齐方案。\n\n依据规则：\n- 尽可能从仓库中引用当前版本和目标版本（锁文件、包清单）\n- 不要猜测版本；如果目标不明确，请提出可选方案并标明为建议"), "Codex", Some(17)),
            ("未测试路径检测", Some("找出近期变更中未测试的路径；补充有针对性的测试，并对草稿 PR 使用 $yeet。\n\n约束：\n- 范围仅限变更区域；避免大范围重构\n- 优先编写小而可靠的测试，确保修改前失败、修改后通过"), "Codex", Some(18)),
            ("打标签前核对", Some("打标签前，请核对变更日志、迁移、功能开关和测试。\n\n依据规则：\n- 仅报告你能从代码库和 CI 上下文中确认的内容\n- 如果某项检查无法验证，请明确标记为\"未知\""), "Codex", Some(19)),
            ("AGENTS.md 更新", Some("用新发现的工作流程和命令更新 AGENTS.md。\n\n约束：\n- 保持改动最小、准确，并以仓库中的实际用法为依据\n- 不要改动无关部分或自动生成的文件\n- 如果不确定，优先添加带简短说明的 TODO，而不是编造内容"), "Codex", Some(20)),
            ("PR 周报总结", Some("按队友和主题总结上周的 PR；突出风险。\n\n依据规则：\n- 有 PR 编号/标题时请使用\n- 不要推测影响；只说明 PR 改了什么"), "Codex", Some(21)),
            ("Issue 分诊", Some("分诊新问题；建议负责人、优先级和标签。\n\n依据规则：\n- 根据问题内容 + 仓库上下文（CODEOWNERS、涉及区域、以往类似问题）给出建议\n- 没有明确信号时不要猜测负责人；如不明确，请写\"Owner: Unknown\"，并建议一个团队"), "Codex", Some(22)),
            ("CI 失败分组分析", Some("检查 CI 失败；按可能的根因分组，并建议最小修复。\n\n依据规则：\n- 引用作业、测试、错误和日志证据\n- 避免过度自信地断定根因；将不确定项标记为\"疑似\""), "Codex", Some(23)),
            ("过时依赖扫描", Some("扫描过时依赖；在尽量少改动的前提下提出安全升级建议。\n\n规则：\n- 优先选择最小可行的升级集\n- 明确指出破坏性变更风险和所需迁移\n- 未先从仓库中识别当前版本，不得提出升级建议"), "Codex", Some(24)),
            ("性能回归审查", Some("审查性能回归，并提出收益最大的修复建议。\n\n依据规则：\n- 有测量数据/跟踪信息时，结论应以其为依据\n- 若证据不足，简要说明不确定性，并建议下一步应测量什么"), "Codex", Some(25)),
            ("变更日志更新", Some("用本周亮点和关键 PR 链接更新变更日志。\n\n约束：\n- 仅包含有仓库历史支持的条目\n- 保持结构简洁，并与现有变更日志格式一致"), "Codex", Some(26)),
            // Codex 应用自动化类（来自 OpenAI 官方文档）
            ("自动创建 Skills", Some("扫描过去一天的 ~/.codex/sessions 文件，如果发现某些 skills 有使用问题，更新这些 skills 使其更有用。\n\n规则：\n- 仅处理个人 skills，不处理仓库 skills\n- 如果有什么我们经常做但需要努力才能完成的事情，且应该保存为 skill 来加速未来工作，那就去做\n- 不觉得必须更新任何内容——只有在有充分理由时才更新！\n- 如果做了任何更改请告诉我"), "Codex应用", Some(27)),
            ("项目动态简报", Some("查看最新的 remote origin/master 或 origin/main。然后为过去 24 小时 touching <DIRECTORY> 的提交生成执行简报。\n\n格式与结构：\n- 使用丰富的 Markdown（H1 工作流标题、斜体副标题、需要时使用水平线）\n- 开头可以写：'Here's the last 24h brief for <directory>:'\n- 副标题：'Narrative walkthrough with owners; grouped by workstream.'\n- 按工作流分组，而非列出每个提交\n- 工作流标题使用 H1\n- 为每个工作流写一段简短的叙述性说明\n- 酌情使用项目符号和粗体\n\n内容要求：\n- 包含内联 PR 链接（例如 [#123](...)），不带 'PRs:' 标签\n- 不包含 commit 哈希或 'Key commits' 部分\n- 范围仅限当前 cwd（或主 checkout 等效目录）内过去 24 小时的提交\n- 使用 gh 获取 PR 标题和描述（如果有帮助的话）\n- 也可以拉取 PR reviews 和 comments"), "Codex应用", Some(28)),
            ("最近代码 Bug 修复", Some("# 最近代码 Bug 修复\n\n## 概述\n\n在当前工作目录中，找到最近一周内由当前作者引入的 bug，实现修复，并在可能的情况下进行验证。确保根本原因直接关联到作者自己的编辑。\n\n## 工作流程\n\n### 1) 确定最近变更范围\n\n使用 Git 识别最近一周内作者的变更文件。从 git config user.name/user.email 确定作者。如果不可用，使用环境中当前用户名或询问一次。使用 git log --since=1.week --author=<author> 列出最近的提交和文件。专注于这些提交触及的文件。如果用户 prompt 为空，直接使用此默认范围。\n\n### 2) 找到与最近变更相关的具体失败\n\n优先查找直接归因于作者编辑的缺陷。如果本地有日志或 CI 输出可用，查找最近的失败（测试、lint、运行时错误）。如果没有提供失败，在编辑的文件上运行最小的相关验证（单个测试、文件级 lint 或有针对性的复现命令）。确认根本原因直接关联到作者的变更，而非无关的遗留问题。如果只找到无关的失败，停止并报告未检测到符合条件的 bug。\n\n### 3) 实现修复\n\n做出符合项目约定的最小修复。只更新解决该问题所需的文件。避免添加额外的防御性检查或无关的重构。保持与本地风格和测试一致。\n\n### 4) 验证\n\n在可能的情况下进行验证。首选最小的验证步骤（有针对性的测试、聚焦的 lint 或直接的复现命令）。如果无法运行验证，说明将运行什么以及为什么没有执行。\n\n### 5) 报告\n\n总结根本原因、修复和执行的验证。明确说明根本原因如何关联到作者最近的变更。"), "Codex应用", Some(29)),
            ("Bug 修复自动化", Some("检查我过去 24 小时的提交，并提交 $recent-code-bugfix。\n\n这是一个调用 skill 的自动化，具体的 bug 修复逻辑在 recent-code-bugfix skill 中定义。"), "Codex应用", Some(30)),
            // Skill 模板（可被自动化调用）
            ("recent-code-bugfix", Some("---\nname: recent-code-bugfix\ndescription: Find and fix a bug introduced by the current author within the last week in the current working directory. Use when a user wants a proactive bugfix from their recent changes, when the prompt is empty, or when asked to triage/fix issues caused by their recent commits. Root cause must map directly to the author's own changes.\n---\n\n# Recent Code Bugfix\n\n## Overview\n\nFind a bug introduced by the current author in the last week, implement a fix, and verify it when possible. Operate in the current working directory, assume the code is local, and ensure the root cause is tied directly to the author's own edits.\n\n## Workflow\n\n### 1) Establish the recent-change scope\n\nUse Git to identify the author and changed files from the last week.\n- Determine the author from `git config user.name`/`user.email`. If unavailable, use the current user's name from the environment or ask once.\n- Use `git log --since=1.week --author=<author>` to list recent commits and files. Focus on files touched by those commits.\n- If the user's prompt is empty, proceed directly with this default scope.\n\n### 2) Find a concrete failure tied to recent changes\n\nPrioritize defects that are directly attributable to the author's edits.\n- Look for recent failures (tests, lint, runtime errors) if logs or CI outputs are available locally.\n- If no failures are provided, run the smallest relevant verification (single test, file-level lint, or targeted repro) that touches the edited files.\n- Confirm the root cause is directly connected to the author's changes, not unrelated legacy issues.\n- If only unrelated failures are found, stop and report that no qualifying bug was detected.\n\n### 3) Implement the fix\n\nMake a minimal fix that aligns with project conventions. Update only the files needed to resolve the issue. Avoid adding extra defensive checks or unrelated refactors. Keep changes consistent with local style and tests.\n\n### 4) Verify\n\nAttempt verification when possible. Prefer the smallest validation step (targeted test, focused lint, or direct repro command). If verification cannot be run, state what would be run and why it wasn't executed.\n\n### 5) Report\n\nSummarize the root cause, the fix, and the verification performed. Make it explicit how the root cause ties to the author's recent changes."), "Codex应用", Some(31)),
        ];

        for (title, prompt, category, sort_order) in default_templates {
            self.create_template(title, prompt, category, sort_order).await?;
        }

        Ok(())
    }
}
