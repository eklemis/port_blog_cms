use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media_attachments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,

    pub media_id: Uuid,

    pub attachable_type: String,
    pub attachable_id: Uuid,

    pub role: String,
    pub position: i32,

    pub alt_text: Option<String>,
    pub caption: Option<String>,

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
