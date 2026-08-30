use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::blog::domain::entities::BlogPost;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blog_posts")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub id: Uuid,

    #[sea_orm(column_name = "user_id", column_type = "Uuid")]
    pub user_id: Uuid,

    #[sea_orm(column_type = "Text", string_len = 200)]
    pub title: String,

    #[sea_orm(column_type = "Text", string_len = 200)]
    pub slug: String,

    #[sea_orm(column_type = "Text", nullable)]
    pub excerpt: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub content: String,

    /// NULL is a draft; a timestamp is the publication moment, which may be in
    /// the future for a scheduled post.
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub published_at: Option<DateTimeWithTimeZone>,

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

    #[sea_orm(has_many = "super::blog_post_topics::Entity")]
    BlogPostTopics,
}

impl Related<super::blog_post_topics::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BlogPostTopics.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(self) -> BlogPost {
        BlogPost {
            id: self.id,
            user_id: self.user_id,
            title: self.title,
            slug: self.slug,
            excerpt: self.excerpt,
            content: self.content,
            published_at: self.published_at.map(|t| t.with_timezone(&chrono::Utc)),
            created_at: self.created_at.with_timezone(&chrono::Utc),
            updated_at: self.updated_at.with_timezone(&chrono::Utc),
        }
    }
}
