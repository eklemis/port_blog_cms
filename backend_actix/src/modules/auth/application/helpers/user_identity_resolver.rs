use std::sync::Arc;

use crate::auth::application::domain::entities::UserId;
use crate::auth::application::ports::outgoing::user_query::{UserQuery, UserQueryError};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ResolveUserIdError {
    #[error("User not found")]
    NotFound,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

#[derive(Clone)]
pub struct UserIdentityResolver {
    user_query: Arc<dyn UserQuery + Send + Sync>,
}

impl UserIdentityResolver {
    pub fn new(user_query: Arc<dyn UserQuery + Send + Sync>) -> Self {
        Self { user_query }
    }

    pub async fn by_username(&self, username: &str) -> Result<UserId, ResolveUserIdError> {
        match self.user_query.find_by_username(username).await {
            Ok(Some(user)) if !user.is_deleted => Ok(UserId::from(user.id)),
            Ok(_) => Err(ResolveUserIdError::NotFound),
            Err(UserQueryError::DatabaseError(msg)) | Err(UserQueryError::QueryFailed(msg)) => {
                Err(ResolveUserIdError::RepositoryError(msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use crate::auth::application::ports::outgoing::user_query::{
        UserQuery, UserQueryError, UserQueryResult,
    };

    /* --------------------------------------------------
     * Mock UserQuery
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockUserQuery {
        result: Result<Option<UserQueryResult>, UserQueryError>,
    }

    impl MockUserQuery {
        fn found(user: UserQueryResult) -> Self {
            Self {
                result: Ok(Some(user)),
            }
        }

        fn not_found() -> Self {
            Self { result: Ok(None) }
        }

        fn error(err: UserQueryError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_id(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!("not used in resolver tests")
        }

        async fn find_by_email(
            &self,
            _email: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!("not used in resolver tests")
        }

        async fn find_by_username(
            &self,
            _username: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            self.result.clone()
        }
    }

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn sample_user(id: Uuid, deleted: bool) -> UserQueryResult {
        UserQueryResult {
            id,
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password_hash: "hashed".to_string(),
            full_name: "Test User".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_verified: true,
            is_deleted: deleted,
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[tokio::test]
    async fn resolves_user_id_when_user_exists_and_not_deleted() {
        let user_id = Uuid::new_v4();
        let query = MockUserQuery::found(sample_user(user_id, false));

        let resolver = UserIdentityResolver::new(Arc::new(query));

        let result = resolver.by_username("testuser").await;

        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved, UserId::from(user_id));
    }

    #[tokio::test]
    async fn returns_not_found_when_user_does_not_exist() {
        let query = MockUserQuery::not_found();
        let resolver = UserIdentityResolver::new(Arc::new(query));

        let result = resolver.by_username("missing").await;

        assert!(matches!(result, Err(ResolveUserIdError::NotFound)));
    }

    #[tokio::test]
    async fn returns_not_found_when_user_is_deleted() {
        let user_id = Uuid::new_v4();
        let query = MockUserQuery::found(sample_user(user_id, true));

        let resolver = UserIdentityResolver::new(Arc::new(query));

        let result = resolver.by_username("deleteduser").await;

        assert!(matches!(result, Err(ResolveUserIdError::NotFound)));
    }

    #[tokio::test]
    async fn maps_database_error_to_repository_error() {
        let query = MockUserQuery::error(UserQueryError::DatabaseError("db down".to_string()));

        let resolver = UserIdentityResolver::new(Arc::new(query));

        let result = resolver.by_username("testuser").await;

        match result {
            Err(ResolveUserIdError::RepositoryError(msg)) => {
                assert!(msg.contains("db down"));
            }
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn maps_query_failed_error_to_repository_error() {
        let query = MockUserQuery::error(UserQueryError::QueryFailed("bad query".to_string()));

        let resolver = UserIdentityResolver::new(Arc::new(query));

        let result = resolver.by_username("testuser").await;

        match result {
            Err(ResolveUserIdError::RepositoryError(msg)) => {
                assert!(msg.contains("bad query"));
            }
            _ => panic!("Expected RepositoryError"),
        }
    }
}
