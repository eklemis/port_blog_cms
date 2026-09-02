//! SeaORM entity for the `blog_post_previews` table — draft preview links.
//!
//! `Model`'s fields mirror the columns one for one. The columns are defined in
//! `migration/`, and what they mean — including why the token is stored as-is —
//! is documented there and on the port DTOs.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blog_post_previews")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub post_id: Uuid,

    #[sea_orm(column_type = "Text", unique)]
    pub token: String,

    pub expires_at: DateTimeWithTimeZone,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::blog_posts::Entity",
        from = "Column::PostId",
        to = "super::blog_posts::Column::Id",
        on_delete = "Cascade"
    )]
    BlogPost,
}

impl Related<super::blog_posts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BlogPost.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
