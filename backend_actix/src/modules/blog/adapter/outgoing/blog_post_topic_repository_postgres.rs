use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::outgoing::{
    BlogPostTopicRepository, BlogPostTopicRepositoryError,
};

#[derive(Clone)]
pub struct BlogPostTopicRepositoryPostgres {
    db: Arc<DatabaseConnection>,
}

impl BlogPostTopicRepositoryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn db_err(e: sea_orm::DbErr) -> BlogPostTopicRepositoryError {
        BlogPostTopicRepositoryError::DatabaseError(e.to_string())
    }

    /// Confirms the post exists, is live, and belongs to the caller.
    ///
    /// Done as its own statement rather than folded into the write, because the
    /// caller needs PostNotFound and TopicNotFound told apart, and a single
    /// INSERT that violates either foreign key cannot say which one it was.
    async fn assert_owns_post(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError> {
        let found = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"SELECT id FROM blog_posts
                   WHERE id = $1 AND user_id = $2 AND is_deleted = false"#,
                [post_id.into(), owner.value().into()],
            ))
            .await
            .map_err(Self::db_err)?;

        found
            .map(|_| ())
            .ok_or(BlogPostTopicRepositoryError::PostNotFound)
    }

    async fn assert_topic_exists(
        &self,
        owner: UserId,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError> {
        let found = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"SELECT id FROM topics
                   WHERE id = $1 AND user_id = $2 AND is_deleted = false"#,
                [topic_id.into(), owner.value().into()],
            ))
            .await
            .map_err(Self::db_err)?;

        found
            .map(|_| ())
            .ok_or(BlogPostTopicRepositoryError::TopicNotFound)
    }
}

#[async_trait]
impl BlogPostTopicRepository for BlogPostTopicRepositoryPostgres {
    async fn attach(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError> {
        self.assert_owns_post(owner, post_id).await?;
        self.assert_topic_exists(owner, topic_id).await?;

        // ON CONFLICT DO NOTHING against the composite primary key, so
        // attaching an already-attached topic is a no-op rather than an error.
        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"INSERT INTO blog_post_topics (blog_post_id, topic_id, created_at)
                   VALUES ($1, $2, NOW())
                   ON CONFLICT (blog_post_id, topic_id) DO NOTHING"#,
                [post_id.into(), topic_id.into()],
            ))
            .await
            .map_err(Self::db_err)?;

        Ok(())
    }

    async fn detach(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError> {
        self.assert_owns_post(owner, post_id).await?;

        // Detaching a topic that is not attached is not an error: the caller
        // asked for it to be gone and it is gone.
        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"DELETE FROM blog_post_topics WHERE blog_post_id = $1 AND topic_id = $2"#,
                [post_id.into(), topic_id.into()],
            ))
            .await
            .map_err(Self::db_err)?;

        Ok(())
    }

    async fn clear(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError> {
        self.assert_owns_post(owner, post_id).await?;

        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"DELETE FROM blog_post_topics WHERE blog_post_id = $1"#,
                [post_id.into()],
            ))
            .await
            .map_err(Self::db_err)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult, Value};
    use std::collections::BTreeMap;

    fn row(id: Uuid) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("id".to_string(), Value::Uuid(Some(Box::new(id))));
        m
    }

    fn exec(n: u64) -> MockExecResult {
        MockExecResult {
            last_insert_id: 0,
            rows_affected: n,
        }
    }

    fn repo(db: DatabaseConnection) -> BlogPostTopicRepositoryPostgres {
        BlogPostTopicRepositoryPostgres::new(Arc::new(db))
    }

    #[tokio::test]
    async fn attach_links_a_topic_the_caller_owns() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![row(Uuid::new_v4())]]) // post ownership
            .append_query_results(vec![vec![row(Uuid::new_v4())]]) // topic exists
            .append_exec_results([exec(1)])
            .into_connection();

        assert!(repo(db)
            .attach(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await
            .is_ok());
    }

    /// The ownership and topic checks are separate statements precisely so
    /// these two errors can be told apart; a single INSERT violating either
    /// foreign key could not say which.
    #[tokio::test]
    async fn attach_reports_post_not_found_before_touching_the_topic() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let err = repo(db)
            .attach(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BlogPostTopicRepositoryError::PostNotFound));
    }

    #[tokio::test]
    async fn attach_reports_topic_not_found_when_the_topic_is_missing() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![row(Uuid::new_v4())]])
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let err = repo(db)
            .attach(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BlogPostTopicRepositoryError::TopicNotFound));
    }

    /// Detaching a topic that was never attached is what the caller asked for,
    /// so zero rows removed is success.
    #[tokio::test]
    async fn detach_is_idempotent() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![row(Uuid::new_v4())]])
            .append_exec_results([exec(0)])
            .into_connection();

        assert!(repo(db)
            .detach(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn detach_requires_owning_the_post() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let err = repo(db)
            .detach(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BlogPostTopicRepositoryError::PostNotFound));
    }

    #[tokio::test]
    async fn clear_removes_every_link() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![row(Uuid::new_v4())]])
            .append_exec_results([exec(3)])
            .into_connection();

        assert!(repo(db)
            .clear(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn clear_requires_owning_the_post() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let err = repo(db)
            .clear(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BlogPostTopicRepositoryError::PostNotFound));
    }
}
