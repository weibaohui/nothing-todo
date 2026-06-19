use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 环路 hook：跨阶段或整环路的 pre/post 钩子。
///
/// hook_position:
/// - pre_loop / post_loop: source_stage_id 必须为 NULL
/// - pre_stage / post_stage: source_stage_id 必填
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "loop_hooks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub loop_id: i64,
    /// pre_loop | post_loop | pre_stage | post_stage
    pub hook_position: String,
    /// 仅 pre_stage/post_stage 必填
    pub source_stage_id: Option<i64>,
    pub target_todo_id: i64,
    #[sea_orm(default_value = "0")]
    pub skip_if_missing: i32,
    #[sea_orm(default_value = "1")]
    pub enabled: i32,
    pub min_rating: Option<i32>,
    #[sea_orm(default_value = "skip")]
    pub unrated_policy: String,
    pub created_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::loops::Entity",
        from = "Column::LoopId",
        to = "super::loops::Column::Id"
    )]
    BelongsToLoop,
    #[sea_orm(
        belongs_to = "super::loop_stages::Entity",
        from = "Column::SourceStageId",
        to = "super::loop_stages::Column::Id"
    )]
    BelongsToSourceStage,
    #[sea_orm(
        belongs_to = "super::todos::Entity",
        from = "Column::TargetTodoId",
        to = "super::todos::Column::Id"
    )]
    BelongsToTargetTodo,
}

impl Related<super::loops::Entity> for Entity {
    fn to() -> RelationDef { Relation::BelongsToLoop.def() }
}

impl Related<super::loop_stages::Entity> for Entity {
    fn to() -> RelationDef { Relation::BelongsToSourceStage.def() }
}

impl Related<super::todos::Entity> for Entity {
    fn to() -> RelationDef { Relation::BelongsToTargetTodo.def() }
}

impl ActiveModelBehavior for ActiveModel {}
