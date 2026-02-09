use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,

    pub user_id: Uuid,

    pub bucket_name: String,
    pub object_key: String,

    pub original_filename: String,
    pub mime_type: String,
    pub file_size_bytes: i64,

    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_seconds: Option<Decimal>,

    pub status: MediaStatus,

    pub metadata: Json,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "media_status")]
pub enum MediaStatus {
    #[sea_orm(string_value = "pending")]
    Pending,

    #[sea_orm(string_value = "processing")]
    Processing,

    #[sea_orm(string_value = "ready")]
    Ready,

    #[sea_orm(string_value = "failed")]
    Failed,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    MediaAttachments,
    MediaVariants,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::MediaAttachments => Entity::has_many(super::media_attachments::Entity).into(),
            Self::MediaVariants => Entity::has_many(super::media_variants::Entity).into(),
        }
    }
}

impl Related<super::media_attachments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MediaAttachments.def()
    }
}

impl Related<super::media_variants::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MediaVariants.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
