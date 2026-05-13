use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};

use crate::db::Database;
use crate::db::entity::todo_templates;
use crate::models::TodoTemplate;

impl Database {
    pub async fn get_template_by_id(&self, id: i64) -> Result<Option<TodoTemplate>, sea_orm::DbErr> {
        let model = todo_templates::Entity::find_by_id(id)
            .one(&self.conn)
            .await?;
        Ok(model.map(|m| TodoTemplate {
            id: m.id,
            title: m.title,
            prompt: m.prompt,
            category: m.category,
            sort_order: m.sort_order.unwrap_or(0),
            is_system: m.is_system,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }))
    }

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
                is_system: m.is_system,
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
                is_system: m.is_system,
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
        is_system: bool,
    ) -> Result<i64, sea_orm::DbErr> {
        let now = crate::models::utc_timestamp();
        let am = todo_templates::ActiveModel {
            title: ActiveValue::Set(title.to_string()),
            prompt: ActiveValue::Set(prompt.map(String::from)),
            category: ActiveValue::Set(category.to_string()),
            sort_order: ActiveValue::Set(sort_order),
            is_system: ActiveValue::Set(is_system),
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
        let default_templates = vec![
            // 开发流程（来自飞书文档）
            ("Bug 扫描与修复", Some("扫描最近的提交（自上次运行以来，或过去 24 小时内），查找可能的 bug 并提出最小修复方案。\n\n依据规则：\n- 只使用仓库中的具体证据（提交 SHA、PR、文件路径、diff、失败的测试、CI 信号）\n- 不要臆造 bug；如果证据不足，请说明并跳过\n- 优先选择最小且安全的修复；避免重构和无关清理"), "开发流程", Some(1)),
            ("发布说明草稿", Some("根据已合并的 PR 起草每周发布说明（如有链接请附上）。\n\n范围与依据：\n- 严格限定在该仓库本周的历史记录内；不要添加超出数据支持的额外章节\n- 使用 PR 编号/标题；除非仓库中的 PR 描述、测试或指标支持，否则避免对影响作出结论"), "开发流程", Some(2)),
            ("站会 Git 活动总结", Some("为站会总结昨天的 git 活动。\n\n依据规则：\n- 陈述需锚定到提交/PR/文件；不要臆测意图或未来工作\n- 保持便于快速浏览，适合团队同步"), "开发流程", Some(3)),
            ("CI 失败总结与建议", Some("总结最近一个 CI 窗口中的 CI 失败和不稳定测试；提出首要修复建议。\n\n依据规则：\n- 尽可能引用具体作业、测试、错误信息或日志片段\n- 避免过度自信地断言根因；区分\"已观察到\"与\"疑似\""), "开发流程", Some(4)),
            ("经典小游戏实现", Some("创建一个范围尽可能小的经典小游戏。\n\n约束：\n- 除非必要，否则不要添加额外功能、样式系统、内容或新的依赖\n- 复用现有仓库的工具和模式"), "开发流程", Some(5)),
            ("性能回归对比", Some("将最近的更改与基准测试或追踪结果进行比较，并尽早标记回归。\n\n依据规则：\n- 以可测量的信号（基准测试、追踪、耗时、火焰图）为依据\n- 如果没有测量数据，请写明\"未找到测量数据\"，不要猜测"), "开发流程", Some(6)),
            ("依赖版本漂移检测", Some("检测依赖项和 SDK 的版本漂移，并提出最小对齐方案。\n\n依据规则：\n- 尽可能从仓库中引用当前版本和目标版本（锁文件、包清单）\n- 不要猜测版本；如果目标不明确，请提出可选方案并标明为建议"), "开发流程", Some(7)),
            ("未测试路径检测", Some("找出近期变更中未测试的路径；补充有针对性的测试，并对草稿 PR 使用 $yeet。\n\n约束：\n- 范围仅限变更区域；避免大范围重构\n- 优先编写小而可靠的测试，确保修改前失败、修改后通过"), "开发流程", Some(8)),
            ("打标签前核对", Some("打标签前，请核对变更日志、迁移、功能开关和测试。\n\n依据规则：\n- 仅报告你能从代码库和 CI 上下文中确认的内容\n- 如果某项检查无法验证，请明确标记为\"未知\""), "开发流程", Some(9)),
            ("AGENTS.md 更新", Some("用新发现的工作流程和命令更新 AGENTS.md。\n\n约束：\n- 保持改动最小、准确，并以仓库中的实际用法为依据\n- 不要改动无关部分或自动生成的文件\n- 如果不确定，优先添加带简短说明的 TODO，而不是编造内容"), "开发流程", Some(10)),
            ("PR 周报总结", Some("按队友和主题总结上周的 PR；突出风险。\n\n依据规则：\n- 有 PR 编号/标题时请使用\n- 不要推测影响；只说明 PR 改了什么"), "开发流程", Some(11)),
            ("Issue 分诊", Some("分诊新问题；建议负责人、优先级和标签。\n\n依据规则：\n- 根据问题内容 + 仓库上下文（CODEOWNERS、涉及区域、以往类似问题）给出建议\n- 没有明确信号时不要猜测负责人；如不明确，请写\"Owner: Unknown\"，并建议一个团队"), "开发流程", Some(12)),
            ("CI 失败分组分析", Some("检查 CI 失败；按可能的根因分组，并建议最小修复。\n\n依据规则：\n- 引用作业、测试、错误和日志证据\n- 避免过度自信地断定根因；将不确定项标记为\"疑似\""), "开发流程", Some(13)),
            ("过时依赖扫描", Some("扫描过时依赖；在尽量少改动的前提下提出安全升级建议。\n\n规则：\n- 优先选择最小可行的升级集\n- 明确指出破坏性变更风险和所需迁移\n- 未先从仓库中识别当前版本，不得提出升级建议"), "开发流程", Some(14)),
            ("性能回归审查", Some("审查性能回归，并提出收益最大的修复建议。\n\n依据规则：\n- 有测量数据/跟踪信息时，结论应以其为依据\n- 若证据不足，简要说明不确定性，并建议下一步应测量什么"), "开发流程", Some(15)),
            ("变更日志更新", Some("用本周亮点和关键 PR 链接更新变更日志。\n\n约束：\n- 仅包含有仓库历史支持的条目\n- 保持结构简洁，并与现有变更日志格式一致"), "开发流程", Some(16)),
            // 自动化任务（来自 OpenAI 官方文档）
            ("自动创建 Skills", Some("扫描过去一天的 ~/.codex/sessions 文件，如果发现某些 skills 有使用问题，更新这些 skills 使其更有用。\n\n规则：\n- 仅处理个人 skills，不处理仓库 skills\n- 如果有什么我们经常做但需要努力才能完成的事情，且应该保存为 skill 来加速未来工作，那就去做\n- 不觉得必须更新任何内容——只有在有充分理由时才更新！\n- 如果做了任何更改请告诉我"), "自动化", Some(17)),
            ("项目动态简报", Some("查看最新的 remote origin/master 或 origin/main。然后为过去 24 小时 touching <DIRECTORY> 的提交生成执行简报。\n\n格式与结构：\n- 使用丰富的 Markdown（H1 工作流标题、斜体副标题、需要时使用水平线）\n- 开头可以写：'Here's the last 24h brief for <directory>:'\n- 副标题：'Narrative walkthrough with owners; grouped by workstream.'\n- 按工作流分组，而非列出每个提交\n- 工作流标题使用 H1\n- 为每个工作流写一段简短的叙述性说明\n- 酌情使用项目符号和粗体\n\n内容要求：\n- 包含内联 PR 链接（例如 [#123](...)），不带 'PRs:' 标签\n- 不包含 commit 哈希或 'Key commits' 部分\n- 范围仅限当前 cwd（或主 checkout 等效目录）内过去 24 小时的提交\n- 使用 gh 获取 PR 标题和描述（如果有帮助的话）\n- 也可以拉取 PR reviews 和 comments"), "自动化", Some(18)),
            ("Bug 修复自动化", Some("检查我过去 24 小时的提交，并提交 $recent-code-bugfix。"), "自动化", Some(19)),
            // Skill 模板
            ("recent-code-bugfix", Some("---\nname: recent-code-bugfix\ndescription: Find and fix a bug introduced by the current author within the last week in the current working directory.\n---\n\n# Recent Code Bugfix\n\n## Overview\n\nFind a bug introduced by the current author in the last week, implement a fix, and verify it when possible.\n\n## Workflow\n\n### 1) Establish the recent-change scope\n\nUse Git to identify the author and changed files from the last week.\n- Determine the author from `git config user.name`/`user.email`. If unavailable, use the current user's name from the environment or ask once.\n- Use `git log --since=1.week --author=<author>` to list recent commits and files. Focus on files touched by those commits.\n\n### 2) Find a concrete failure tied to recent changes\n\nPrioritize defects that are directly attributable to the author's edits.\n- Look for recent failures (tests, lint, runtime errors) if logs or CI outputs are available locally.\n- If no failures are provided, run the smallest relevant verification (single test, file-level lint, or targeted repro) that touches the edited files.\n- Confirm the root cause is directly connected to the author's changes.\n\n### 3) Implement the fix\n\nMake a minimal fix that aligns with project conventions. Update only the files needed to resolve the issue.\n\n### 4) Verify\n\nAttempt verification when possible. Prefer the smallest validation step (targeted test, focused lint, or direct repro command).\n\n### 5) Report\n\nSummarize the root cause, the fix, and the verification performed."), "自动化", Some(20)),
        ];

        for (title, prompt, category, sort_order) in default_templates {
            // Check if this system template already exists
            let existing = todo_templates::Entity::find()
                .filter(todo_templates::Column::Title.eq(title.to_string()))
                .filter(todo_templates::Column::IsSystem.eq(true))
                .one(&self.conn)
                .await?;

            if let Some(model) = existing {
                // Update existing system template
                let mut am: todo_templates::ActiveModel = model.into();
                am.prompt = ActiveValue::Set(prompt.map(String::from));
                am.category = ActiveValue::Set(category.to_string());
                am.sort_order = ActiveValue::Set(sort_order);
                am.updated_at = ActiveValue::Set(Some(crate::models::utc_timestamp()));
                am.update(&self.conn).await?;
            } else {
                // Insert new system template
                self.create_template(title, prompt, category, sort_order, true).await?;
            }
        }

        Ok(())
    }
}
