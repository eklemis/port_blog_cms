use async_trait::async_trait;

use crate::auth::application::{
    domain::entities::UserId,
    ports::outgoing::UserQuery,
    use_cases::fetch_profile::{FetchUserError, FetchUserOutput, FetchUserProfileUseCase},
};

pub struct FetchUserProfileService<Q>
where
    Q: UserQuery + Send + Sync,
{
    user_query: Q,
}

impl<Q> FetchUserProfileService<Q>
where
    Q: UserQuery + Send + Sync,
{
    pub fn new(query: Q) -> Self {
        Self { user_query: query }
    }
}

#[async_trait]
impl<Q> FetchUserProfileUseCase for FetchUserProfileService<Q>
where
    Q: UserQuery + Send + Sync,
{
    async fn execute(&self, user_id: UserId) -> Result<FetchUserOutput, FetchUserError> {
        let user = self
            .user_query
            .find_by_id(user_id.value())
            .await?
            .ok_or_else(|| FetchUserError::UserNotFound(format!("{}", user_id.value())))?;

        Ok(FetchUserOutput {
            user_id: user.id.into(),
            email: user.email,
            username: user.username,
            full_name: user.full_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::user_query::{UserQueryError, UserQueryResult};
    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    struct MockUserQuery {
        result: Result<Option<UserQueryResult>, UserQueryError>,
    }

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_id(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            self.result.clone()
        }

        async fn find_by_email(
            &self,
            _email: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }

        async fn find_by_username(
            &self,
            _username: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
    }

    fn create_user_query_result(id: Uuid) -> UserQueryResult {
        UserQueryResult {
            id,
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password_hash: "hashed".to_string(),
            full_name: "Test User".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_verified: true,
            is_deleted: false,
        }
    }

    #[tokio::test]
    async fn test_execute_success() {
        let user_id = Uuid::new_v4();
        let query_result = create_user_query_result(user_id);

        let mock_query = MockUserQuery {
            result: Ok(Some(query_result.clone())),
        };

        let service = FetchUserProfileService::new(mock_query);
        let result = service.execute(user_id.into()).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.user_id.value(), user_id);
        assert_eq!(output.email, "test@example.com");
        assert_eq!(output.username, "testuser");
        assert_eq!(output.full_name, "Test User");
    }

    #[tokio::test]
    async fn test_execute_user_not_found() {
        let user_id = Uuid::new_v4();

        let mock_query = MockUserQuery { result: Ok(None) };

        let service = FetchUserProfileService::new(mock_query);
        let result = service.execute(user_id.into()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, FetchUserError::UserNotFound(_)));
        assert!(error.to_string().contains(&user_id.to_string()));
    }

    #[tokio::test]
    async fn test_execute_query_error() {
        let user_id = Uuid::new_v4();

        let mock_query = MockUserQuery {
            result: Err(UserQueryError::DatabaseError(
                "Connection failed".to_string(),
            )),
        };

        let service = FetchUserProfileService::new(mock_query);
        let result = service.execute(user_id.into()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, FetchUserError::QueryError(_)));
    }
}
