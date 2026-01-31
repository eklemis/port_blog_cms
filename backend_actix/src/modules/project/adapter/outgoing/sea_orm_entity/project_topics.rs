use crate::project::adapter::outgoing::sea_orm_entity::projects;
use crate::topic::adapter::outgoing::sea_orm_entity::topics;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "project_topics")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub id: Uuid,

    #[sea_orm(column_name = "project_id", column_type = "Uuid")]
    pub project_id: Uuid,

    #[sea_orm(column_name = "topic_id", column_type = "Uuid")]
    pub topic_id: Uuid,

    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::projects::Entity",
        from = "Column::ProjectId",
        to = "super::projects::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    Projects,

    #[sea_orm(
        belongs_to = "crate::topic::adapter::outgoing::sea_orm_entity::topics::Entity",
        from = "Column::TopicId",
        to = "crate::topic::adapter::outgoing::sea_orm_entity::topics::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    Topics,
}

impl Related<projects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Projects.def()
    }
}

impl Related<topics::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Topics.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
