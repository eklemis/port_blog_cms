//! SeaORM entity for the `applications` table.
//!
//! `Model`'s fields mirror the columns one for one. The columns are defined in
//! `migration/`, and what they mean is documented on the domain entity.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "applications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub id: Uuid,

    #[sea_orm(column_type = "Uuid")]
    pub user_id: Uuid,

    #[sea_orm(column_type = "Uuid")]
    pub job_id: Uuid,

    #[sea_orm(column_type = "Uuid", nullable)]
    pub cv_snapshot_id: Option<Uuid>,

    #[sea_orm(column_type = "Text")]
    pub status: String,

    pub applied_at: Option<DateTimeWithTimeZone>,

    #[sea_orm(column_type = "Text")]
    pub next_action: String,

    pub next_action_at: Option<DateTimeWithTimeZone>,

    pub is_deleted: bool,

    pub created_at: DateTimeWithTimeZone,

    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
