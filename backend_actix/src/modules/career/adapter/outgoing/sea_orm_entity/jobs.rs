//! SeaORM entity for the `jobs` table — captured postings.
//!
//! `Model`'s fields mirror the columns one for one. The columns are defined in
//! `migration/`, and what they mean is documented on the domain entity.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub id: Uuid,

    #[sea_orm(column_type = "Uuid")]
    pub user_id: Uuid,

    #[sea_orm(column_type = "Text")]
    pub title: String,

    #[sea_orm(column_type = "Text")]
    pub company: String,

    #[sea_orm(column_type = "Text")]
    pub location: String,

    #[sea_orm(column_type = "Text")]
    pub seniority: String,

    pub required_skills: Json,

    pub nice_to_have: Json,

    #[sea_orm(column_type = "Text")]
    pub source_url: String,

    #[sea_orm(column_type = "Text")]
    pub source_text: String,

    pub is_deleted: bool,

    pub created_at: DateTimeWithTimeZone,

    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
