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
            ("代码审查", Some("审查以下代码，检查潜在 bug 和改进点：\n\n```\n[粘贴代码]\n```"), "开发", Some(1)),
            ("Bug 修复", Some("定位并修复以下问题：\n\n[描述问题]"), "开发", Some(2)),
            ("功能开发", Some("实现以下功能需求：\n\n[描述需求]"), "开发", Some(3)),
            ("代码重构", Some("重构以下代码，提升可读性和性能：\n\n```\n[粘贴代码]\n```"), "开发", Some(4)),
            ("性能优化", Some("分析并优化以下代码的性能：\n\n```\n[粘贴代码]\n```"), "开发", Some(5)),
            ("安全审计", Some("检查以下代码的安全漏洞：\n\n```\n[粘贴代码]\n```"), "安全", Some(6)),
            ("单元测试", Some("为以下代码编写单元测试：\n\n```\n[粘贴代码]\n```"), "测试", Some(7)),
            ("文档撰写", Some("为以下功能编写文档：\n\n[描述功能]"), "文档", Some(8)),
            ("需求分析", Some("分析以下需求并输出技术方案：\n\n[描述需求]"), "需求", Some(9)),
            ("代码解释", Some("解释以下代码的功能和实现原理：\n\n```\n[粘贴代码]\n```"), "学习", Some(10)),
        ];

        for (title, prompt, category, sort_order) in default_templates {
            self.create_template(title, prompt, category, sort_order).await?;
        }

        Ok(())
    }
}
