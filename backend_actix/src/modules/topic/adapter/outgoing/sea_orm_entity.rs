use crate::auth::application::domain::entities::UserId;
use crate::modules::topic::application::ports::outgoing::TopicQueryResult;
use crate::modules::topic::application::ports::outgoing::TopicResult;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "topics")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,

    pub user_id: Uuid,

    pub title: String,

    pub description: Option<String>,

    pub is_deleted: bool,

    pub created_at: DateTimeWithTimeZone,

    pub updated_at: DateTimeWithTimeZone,
}

impl Model {
    pub fn to_repository_result(&self) -> TopicResult {
        TopicResult {
            id: self.id,
            owner: UserId::from(self.user_id),
            title: self.title.clone(),
            description: self.description.clone().unwrap_or(String::from("")),
        }
    }
    pub fn to_query_result(&self) -> TopicQueryResult {
        TopicQueryResult {
            id: self.id,
            owner: UserId::from(self.user_id),
            title: self.title.clone(),
            description: self.description.clone().unwrap_or(String::from("")),
            created_at: self.created_at.into(),
            updated_at: self.updated_at.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::modules::auth::adapter::outgoing::sea_orm_entity::users::Entity",
        from = "Column::UserId",
        to = "crate::modules::auth::adapter::outgoing::sea_orm_entity::users::Column::Id"
    )]
    User,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
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
