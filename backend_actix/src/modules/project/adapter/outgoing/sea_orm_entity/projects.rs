use crate::modules::topic::adapter::outgoing::sea_orm_entity::topics;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue, Set};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub id: Uuid,

    #[sea_orm(column_name = "user_id", column_type = "Uuid")]
    pub user_id: Uuid,

    #[sea_orm(column_type = "Text", string_len = 150)]
    pub title: String,

    #[sea_orm(column_type = "Text", string_len = 150)]
    pub slug: String,

    #[sea_orm(column_type = "Text")]
    pub description: String,

    // Stored as JSONB (you decided to store array in JSONB)
    #[sea_orm(column_type = "JsonBinary")]
    pub tech_stack: Vec<String>,

    #[sea_orm(column_type = "JsonBinary")]
    pub screenshots: Json,

    #[sea_orm(column_type = "Text", nullable)]
    pub repo_url: Option<String>,

    #[sea_orm(column_type = "Text", nullable)]
    pub live_demo_url: Option<String>,

    // Needed for soft_delete + restore
    pub is_deleted: bool,

    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::modules::auth::adapter::outgoing::sea_orm_entity::users::Entity",
        from = "Column::UserId",
        to = "crate::modules::auth::adapter::outgoing::sea_orm_entity::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Users,

    #[sea_orm(
        has_many = "crate::modules::project::adapter::outgoing::sea_orm_entity::project_topics::Entity"
    )]
    ProjectTopics,
}

impl Related<crate::modules::auth::adapter::outgoing::sea_orm_entity::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

// Many-to-many: projects <-> topics via project_topics
impl Related<topics::Entity> for Entity {
    fn to() -> RelationDef {
        crate::modules::project::adapter::outgoing::sea_orm_entity::project_topics::Relation::Topics
            .def()
    }

    fn via() -> Option<RelationDef> {
        Some(
            crate::modules::project::adapter::outgoing::sea_orm_entity::project_topics::Relation::Projects
                .def()
                .rev(),
        )
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if let ActiveValue::Set(slug) = &self.slug {
            self.slug = Set(slug.trim().to_lowercase());
        }

        if let ActiveValue::Set(title) = &self.title {
            self.title = Set(title.trim().to_string());
        }

        #[cfg(feature = "no_db_triggers")]
        {
            use chrono::Utc;
            use sea_orm::ActiveValue::Set;

            let insert = _insert;
            if !insert {
                self.updated_at = Set(Utc::now().into());
            }
        }

        Ok(self)
    }
}
