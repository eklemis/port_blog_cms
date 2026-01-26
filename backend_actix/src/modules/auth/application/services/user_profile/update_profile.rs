use crate::auth::application::{
    ports::outgoing::UserRepository,
    use_cases::update_profile::{
        UpdateUserError, UpdateUserInput, UpdateUserOutput, UpdateUserProfileUseCase,
    },
};
use async_trait::async_trait;

pub struct UpdateUserProfileService<R>
where
    R: UserRepository + Send + Sync,
{
    user_repository: R,
}

impl<R> UpdateUserProfileService<R>
where
    R: UserRepository + Send + Sync,
{
    pub fn new(repository: R) -> Self {
        Self {
            user_repository: repository,
        }
    }

    fn validate_full_name(&self, full_name: &str) -> Result<String, UpdateUserError> {
        let trimmed = full_name.trim();

        if trimmed.is_empty() {
            return Err(UpdateUserError::InvalidFullName(
                "Full name cannot be empty".to_string(),
            ));
        }

        if trimmed.len() < 2 || trimmed.len() > 100 {
            return Err(UpdateUserError::InvalidFullName(
                "Full name must be 2-100 characters".to_string(),
            ));
        }

        Ok(trimmed.to_string())
    }
}

#[async_trait]
impl<R> UpdateUserProfileUseCase for UpdateUserProfileService<R>
where
    R: UserRepository + Send + Sync,
{
    async fn execute(&self, data: UpdateUserInput) -> Result<UpdateUserOutput, UpdateUserError> {
        let full_name = self.validate_full_name(&data.full_name)?;

        let user = self
            .user_repository
            .set_full_name(data.user_id.value(), full_name)
            .await?;

        Ok(UpdateUserOutput {
            user_id: user.id.into(),
            username: user.username,
            email: user.email,
            full_name: user.full_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::{
        ports::outgoing::user_repository::{
            CreateUserData, UserRepository, UserRepositoryError, UserResult,
        },
        use_cases::update_profile::UpdateUserError,
    };
    use async_trait::async_trait;
    use uuid::Uuid;

    struct MockUserRepository {
        result: Result<UserResult, UserRepositoryError>,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn create_user(
            &self,
            _data: CreateUserData,
        ) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }

        async fn restore_user(&self, _user_id: Uuid) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }

        async fn activate_user(&self, _user_id: Uuid) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }

        async fn set_full_name(
            &self,
            _user_id: Uuid,
            _full_name: String,
        ) -> Result<UserResult, UserRepositoryError> {
            self.result.clone()
        }

        async fn update_password(
            &self,
            _user_id: Uuid,
            _new_password_hash: String,
        ) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }

        async fn delete_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }

        async fn soft_delete_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }
    }

    fn create_user_result(id: Uuid, full_name: &str) -> UserResult {
        UserResult {
            id,
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            full_name: full_name.to_string(),
        }
    }

    fn create_update_input(user_id: Uuid, full_name: &str) -> UpdateUserInput {
        UpdateUserInput {
            user_id: user_id.into(),
            full_name: full_name.to_string(),
        }
    }

    #[tokio::test]
    async fn test_execute_success() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, "John Doe")),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "John Doe");

        let result = service.execute(input).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.user_id.value(), user_id);
        assert_eq!(output.full_name, "John Doe");
        assert_eq!(output.email, "test@example.com");
        assert_eq!(output.username, "testuser");
    }

    #[tokio::test]
    async fn test_execute_trims_whitespace() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, "John Doe")),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "  John Doe  ");

        let result = service.execute(input).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_full_name_empty() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, "")),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "");

        let result = service.execute(input).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, UpdateUserError::InvalidFullName(_)));
        assert!(error.to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_execute_full_name_whitespace_only() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, "")),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "   ");

        let result = service.execute(input).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, UpdateUserError::InvalidFullName(_)));
        assert!(error.to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_execute_full_name_too_short() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, "A")),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "A");

        let result = service.execute(input).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, UpdateUserError::InvalidFullName(_)));
        assert!(error.to_string().contains("2-100 characters"));
    }

    #[tokio::test]
    async fn test_execute_full_name_too_long() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, "")),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let long_name = "A".repeat(101);
        let input = create_update_input(user_id, &long_name);

        let result = service.execute(input).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, UpdateUserError::InvalidFullName(_)));
        assert!(error.to_string().contains("2-100 characters"));
    }

    #[tokio::test]
    async fn test_execute_full_name_boundary_min_valid() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, "Jo")),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "Jo");

        let result = service.execute(input).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().full_name, "Jo");
    }

    #[tokio::test]
    async fn test_execute_full_name_boundary_max_valid() {
        let user_id = Uuid::new_v4();
        let max_name = "A".repeat(100);
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, &max_name)),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, &max_name);

        let result = service.execute(input).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().full_name.len(), 100);
    }

    #[tokio::test]
    async fn test_execute_user_not_found() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Err(UserRepositoryError::UserNotFound),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "John Doe");

        let result = service.execute(input).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, UpdateUserError::RepositoryError(_)));
    }

    #[tokio::test]
    async fn test_execute_database_error() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Err(UserRepositoryError::DatabaseError(
                "Connection failed".to_string(),
            )),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "John Doe");

        let result = service.execute(input).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, UpdateUserError::RepositoryError(_)));
    }
}
