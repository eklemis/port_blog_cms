use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::topic::application::ports::outgoing::{
    CreateTopicData, TopicRepository, TopicRepositoryError, TopicResult,
};

// SeaORM entity imports
use super::sea_orm_entity::{ActiveModel as TopicActiveModel, Model as TopicModel};

#[derive(Debug, Clone)]
pub struct TopicRepositoryPostgres {
    db: Arc<DatabaseConnection>,
}

impl TopicRepositoryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TopicRepository for TopicRepositoryPostgres {
    async fn create_topic(
        &self,
        data: CreateTopicData,
    ) -> Result<TopicResult, TopicRepositoryError> {
        let active = TopicActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(data.owner.into()),
            title: Set(data.title),
            description: Set(Some(data.description)),
            is_deleted: Set(false),
            ..Default::default()
        };

        let inserted: TopicModel = active
            .insert(&*self.db)
            .await
            .map_err(|e| TopicRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted.to_repository_result())
    }

    async fn restore_topic(&self, topic_id: Uuid) -> Result<TopicResult, TopicRepositoryError> {
        let active = TopicActiveModel {
            id: Set(topic_id),
            is_deleted: Set(false),
            ..Default::default()
        };

        let result = active
            .update(&*self.db)
            .await
            .map_err(|e| TopicRepositoryError::DatabaseError(e.to_string()))?;

        if result.is_deleted {
            // Should never happen, but safe
            return Err(TopicRepositoryError::TopicNotFound);
        }

        Ok(result.to_repository_result())
    }

    async fn soft_delete_topic(&self, topic_id: Uuid) -> Result<(), TopicRepositoryError> {
        let active = TopicActiveModel {
            id: Set(topic_id),
            is_deleted: Set(true),
            ..Default::default()
        };

        let result = active
            .update(&*self.db)
            .await
            .map_err(|e| TopicRepositoryError::DatabaseError(e.to_string()))?;

        if !result.is_deleted {
            return Err(TopicRepositoryError::TopicNotFound);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult, RuntimeErr};

    fn create_test_topic_model(
        id: Uuid,
        owner: UserId,
        title: &str,
        description: Option<&str>,
        is_deleted: bool,
    ) -> TopicModel {
        let now = Utc::now().fixed_offset();

        TopicModel {
            id,
            user_id: owner.into(),
            title: title.to_string(),
            description: description.map(|d| d.to_string()),
            is_deleted,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_create_topic_success() {
        let topic_id = Uuid::new_v4();
        let owner = UserId::from(Uuid::new_v4());

        let input = CreateTopicData {
            owner,
            title: "Rust".to_string(),
            description: "Rust topic".to_string(),
        };

        let inserted_model =
            create_test_topic_model(topic_id, owner, "Rust", Some("Rust topic"), false);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![inserted_model.clone()]])
            .into_connection();

        let repo = TopicRepositoryPostgres::new(Arc::new(db));

        let result = repo.create_topic(input).await;

        assert!(result.is_ok());
        let topic = result.unwrap();

        assert_eq!(topic.id, topic_id);
        assert_eq!(topic.owner, owner);
        assert_eq!(topic.title, "Rust");
        assert_eq!(topic.description, "Rust topic");
    }

    #[tokio::test]
    async fn test_create_topic_database_error() {
        let owner = UserId::from(Uuid::new_v4());

        let input = CreateTopicData {
            owner,
            title: "Fail".to_string(),
            description: "Fail".to_string(),
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_errors(vec![sea_orm::DbErr::Query(RuntimeErr::Internal(
                "insert failed".into(),
            ))])
            .into_connection();

        let repo = TopicRepositoryPostgres::new(Arc::new(db));

        let result = repo.create_topic(input).await;

        assert!(matches!(
            result,
            Err(TopicRepositoryError::DatabaseError(_))
        ));
    }

    #[tokio::test]
    async fn test_restore_topic_success() {
        let topic_id = Uuid::new_v4();
        let owner = UserId::from(Uuid::new_v4());

        let restored_model = create_test_topic_model(topic_id, owner, "Recovered", None, false);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // update() → exec
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            // returning updated row
            .append_query_results(vec![vec![restored_model.clone()]])
            .into_connection();

        let repo = TopicRepositoryPostgres::new(Arc::new(db));

        let result = repo.restore_topic(topic_id).await;

        assert!(result.is_ok());
        let topic = result.unwrap();

        assert_eq!(topic.id, topic_id);
        assert_eq!(topic.owner, owner);
        assert_eq!(topic.title, "Recovered");
    }

    #[tokio::test]
    async fn test_restore_topic_not_found() {
        let topic_id = Uuid::new_v4();
        let owner = UserId::from(Uuid::new_v4());

        // Simulate "not found" by returning a model that is still deleted
        let deleted_model = TopicModel {
            id: topic_id,
            user_id: owner.into(),
            title: "x".into(),
            description: None,
            is_deleted: true,
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![deleted_model]])
            .into_connection();

        let repo = TopicRepositoryPostgres::new(Arc::new(db));

        let result = repo.restore_topic(topic_id).await;

        assert!(matches!(result, Err(TopicRepositoryError::TopicNotFound)));
    }

    #[tokio::test]
    async fn test_soft_delete_topic_success() {
        let topic_id = Uuid::new_v4();
        let owner = UserId::from(Uuid::new_v4());

        let deleted_model = TopicModel {
            id: topic_id,
            user_id: owner.into(),
            title: "Rust".into(),
            description: None,
            is_deleted: true,
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![deleted_model]])
            .into_connection();

        let repo = TopicRepositoryPostgres::new(Arc::new(db));

        let result = repo.soft_delete_topic(topic_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_soft_delete_topic_not_found() {
        let topic_id = Uuid::new_v4();
        let owner = UserId::from(Uuid::new_v4());

        // Not deleted → update did not apply
        let unchanged_model = TopicModel {
            id: topic_id,
            user_id: owner.into(),
            title: "Rust".into(),
            description: None,
            is_deleted: false,
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![unchanged_model]])
            .into_connection();

        let repo = TopicRepositoryPostgres::new(Arc::new(db));

        let result = repo.soft_delete_topic(topic_id).await;

        assert!(matches!(result, Err(TopicRepositoryError::TopicNotFound)));
    }

    #[test]
    fn test_repository_is_cloneable() {
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let repo = TopicRepositoryPostgres::new(Arc::new(db));

        let _ = repo.clone();
        assert!(true);
    }
}
