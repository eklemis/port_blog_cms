use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media_variants")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,

    pub media_id: Uuid,

    pub variant_type: String,

    pub bucket_name: String,
    pub object_key: String,

    pub mime_type: String,
    pub file_size_bytes: i64,

    pub width: Option<i32>,
    pub height: Option<i32>,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Media,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Media => Entity::belongs_to(super::media::Entity)
                .from(Column::MediaId)
                .to(super::media::Column::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
                .into(),
        }
    }
}

impl Related<super::media::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Media.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
