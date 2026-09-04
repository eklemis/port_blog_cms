use crate::auth::application::{
    ports::outgoing::UserRepository,
    use_cases::update_profile::{
        UpdateUserError, UpdateUserInput, UpdateUserOutput, UpdateUserProfileUseCase,
    },
};
use async_trait::async_trait;

/// Implements the corresponding use-case contract.
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
    /// Builds it from the ports it depends on.
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
            .set_profile(data.user_id.value(), full_name, data.bio, data.locale)
            .await?;

        Ok(UpdateUserOutput {
            user_id: user.id.into(),
            username: user.username,
            email: user.email,
            full_name: user.full_name,
            bio: user.bio,
            locale: user.locale,
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
    use std::sync::Mutex;
    use uuid::Uuid;

    struct MockUserRepository {
        result: Result<UserResult, UserRepositoryError>,
        /// The tri-state bio the service actually handed to the port.
        bio_seen: Mutex<Option<Option<Option<String>>>>,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn set_profile(
            &self,
            _user_id: Uuid,
            _full_name: String,
            bio: Option<Option<String>>,
            _locale: Option<String>,
        ) -> Result<UserResult, UserRepositoryError> {
            *self.bio_seen.lock().unwrap() = Some(bio);
            self.result.clone()
        }

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
            bio: None,
            locale: "en".to_string(),
        }
    }

    fn create_update_input(user_id: Uuid, full_name: &str) -> UpdateUserInput {
        UpdateUserInput {
            user_id: user_id.into(),
            full_name: full_name.to_string(),
            bio: None,
            locale: None,
        }
    }

    #[tokio::test]
    async fn test_execute_success() {
        let user_id = Uuid::new_v4();
        let mock_repo = MockUserRepository {
            result: Ok(create_user_result(user_id, "John Doe")),
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
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
            bio_seen: Mutex::new(None),
        };

        let service = UpdateUserProfileService::new(mock_repo);
        let input = create_update_input(user_id, "John Doe");

        let result = service.execute(input).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, UpdateUserError::RepositoryError(_)));
    }

    // ------------------------------------------------------------------
    // Tri-state bio
    //
    // The service is a pass-through here, so what these pin is that it does
    // not collapse the three states on the way to the port. Folding "absent"
    // into "null" would silently wipe a bio on every name-only edit.
    // ------------------------------------------------------------------

    async fn bio_reaching_the_port(bio: Option<Option<String>>) -> Option<Option<Option<String>>> {
        let user_id = Uuid::new_v4();
        let service = UpdateUserProfileService::new(MockUserRepository {
            result: Ok(create_user_result(user_id, "John Doe")),
            bio_seen: Mutex::new(None),
        });

        service
            .execute(UpdateUserInput {
                user_id: user_id.into(),
                full_name: "John Doe".to_string(),
                bio,
                locale: None,
            })
            .await
            .unwrap();

        let seen = service.user_repository.bio_seen.lock().unwrap();
        seen.clone()
    }

    #[tokio::test]
    async fn an_omitted_bio_is_passed_through_as_leave_alone() {
        assert_eq!(bio_reaching_the_port(None).await, Some(None));
    }

    #[tokio::test]
    async fn an_explicit_null_bio_is_passed_through_as_clear() {
        assert_eq!(bio_reaching_the_port(Some(None)).await, Some(Some(None)));
    }

    #[tokio::test]
    async fn a_new_bio_is_passed_through_verbatim() {
        assert_eq!(
            bio_reaching_the_port(Some(Some("Rust, mostly.".to_string()))).await,
            Some(Some(Some("Rust, mostly.".to_string())))
        );
    }

    /// The edited bio must come back in the response, or a client cannot
    /// confirm the write without a second request.
    #[tokio::test]
    async fn the_response_carries_the_stored_bio() {
        let user_id = Uuid::new_v4();
        let mut stored = create_user_result(user_id, "John Doe");
        stored.bio = Some("Rust, mostly.".to_string());

        let service = UpdateUserProfileService::new(MockUserRepository {
            result: Ok(stored),
            bio_seen: Mutex::new(None),
        });

        let output = service
            .execute(create_update_input(user_id, "John Doe"))
            .await
            .unwrap();

        assert_eq!(output.bio.as_deref(), Some("Rust, mostly."));
    }
}
