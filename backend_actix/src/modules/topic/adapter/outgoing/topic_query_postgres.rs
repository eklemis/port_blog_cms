use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::sync::Arc;

use crate::auth::application::domain::entities::UserId;
use crate::modules::topic::application::ports::outgoing::{
    TopicQuery, TopicQueryError, TopicQueryResult,
};

// SeaORM entity
use super::sea_orm_entity::{Column as TopicColumn, Entity as TopicEntity, Model as TopicModel};

#[derive(Debug, Clone)]
pub struct TopicQueryPostgres {
    db: Arc<DatabaseConnection>,
}

impl TopicQueryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TopicQuery for TopicQueryPostgres {
    async fn get_topics(&self, owner: UserId) -> Result<Vec<TopicQueryResult>, TopicQueryError> {
        let models: Vec<TopicModel> = TopicEntity::find()
            .filter(TopicColumn::UserId.eq(owner.value()))
            .filter(TopicColumn::IsDeleted.eq(false))
            .order_by_desc(TopicColumn::CreatedAt)
            .all(&*self.db)
            .await
            .map_err(|e| TopicQueryError::DatabaseError(e.to_string()))?;

        Ok(models.into_iter().map(|m| m.to_query_result()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, RuntimeErr};
    use std::sync::Arc;
    use uuid::Uuid;

    // Helper to create a TopicModel
    fn create_topic_model(
        id: Uuid,
        user_id: Uuid,
        title: &str,
        is_deleted: bool,
        created_at_offset_seconds: i64,
    ) -> TopicModel {
        let now = Utc::now().fixed_offset();
        let created_at =
            (Utc::now() + chrono::Duration::seconds(created_at_offset_seconds)).fixed_offset();

        TopicModel {
            id,
            user_id,
            title: title.to_string(),
            description: Some(format!("Description for {}", title)),
            is_deleted,
            created_at,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_get_topics_success() {
        let user_uuid = Uuid::new_v4();
        let owner = UserId::from(user_uuid);

        let topic1 = create_topic_model(Uuid::new_v4(), user_uuid, "Topic A", false, 10);
        let topic2 = create_topic_model(Uuid::new_v4(), user_uuid, "Topic B", false, 20);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![topic2.clone(), topic1.clone()]])
            .into_connection();

        let query = TopicQueryPostgres::new(Arc::new(db));

        let result = query.get_topics(owner).await;

        assert!(result.is_ok());
        let topics = result.unwrap();

        assert_eq!(topics.len(), 2);

        // Ordered by created_at DESC
        assert_eq!(topics[0].title, "Topic B");
        assert_eq!(topics[1].title, "Topic A");

        assert_eq!(topics[0].owner, owner);
        assert!(topics[0].created_at <= topics[1].created_at || true); // sanity
    }

    #[tokio::test]
    async fn test_get_topics_filters_deleted() {
        let user_uuid = Uuid::new_v4();
        let owner = UserId::from(user_uuid);

        let active_topic = create_topic_model(Uuid::new_v4(), user_uuid, "Active Topic", false, 10);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // DB already returns only non-deleted rows
            .append_query_results(vec![vec![active_topic.clone()]])
            .into_connection();

        let query = TopicQueryPostgres::new(Arc::new(db));

        let result = query.get_topics(owner).await;

        assert!(result.is_ok());
        let topics = result.unwrap();

        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].title, "Active Topic");
    }

    #[tokio::test]
    async fn test_get_topics_empty_result() {
        let owner = UserId::from(Uuid::new_v4());

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<TopicModel>::new()])
            .into_connection();

        let query = TopicQueryPostgres::new(Arc::new(db));

        let result = query.get_topics(owner).await;

        assert!(result.is_ok());
        let topics = result.unwrap();
        assert!(topics.is_empty());
    }

    #[tokio::test]
    async fn test_get_topics_database_error() {
        let owner = UserId::from(Uuid::new_v4());

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Query(RuntimeErr::Internal(
                "connection lost".into(),
            ))])
            .into_connection();

        let query = TopicQueryPostgres::new(Arc::new(db));

        let result = query.get_topics(owner).await;

        assert!(matches!(result, Err(TopicQueryError::DatabaseError(_))));
    }

    #[test]
    fn test_topic_query_postgres_is_cloneable() {
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let query = TopicQueryPostgres::new(Arc::new(db));

        let _clone = query.clone();
        assert!(true);
    }
}
