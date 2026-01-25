use crate::auth::application::ports::outgoing::user_query::UserQueryError;
use crate::auth::application::ports::outgoing::user_repository::{CreateUserData, UserResult};

use crate::modules::auth::application::ports::outgoing::{
    user_query::UserQuery, user_repository::UserRepository, UserRepositoryError,
};
use async_trait::async_trait;
use email_address::EmailAddress;

use crate::auth::application::ports::outgoing::password_hasher::{HashError, PasswordHasher};
use std::sync::Arc;

// ============================================================================
// Input / Output DTOs
// ============================================================================
#[derive(Clone, Debug)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub full_name: String,
}
#[derive(Clone, Debug)]
pub struct CreateUserOutput {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub username: String,
    pub full_name: String,
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, thiserror::Error, Clone)]
pub enum CreateUserError {
    #[error("Invalid username: {0}")]
    InvalidUsername(String),

    #[error("Invalid email: {0}")]
    InvalidEmail(String),

    #[error("Invalid password: {0}")]
    InvalidPassword(String),

    #[error("Invalid full name: {0}")]
    InvalidFullName(String),

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Password hashing failed: {0}")]
    HashingFailed(String),

    #[error("Repository error: {0}")]
    RepositoryError(#[from] UserRepositoryError),

    #[error("Query error: {0}")]
    QueryError(#[from] UserQueryError),
}

// ============================================================================
// Use Case Interface
// ============================================================================

#[async_trait]
pub trait ICreateUserUseCase: Send + Sync {
    async fn execute(&self, input: CreateUserInput) -> Result<CreateUserOutput, CreateUserError>;
}

// ============================================================================
// Use Case Implementation - FOCUSED ON ONE THING
// ============================================================================

pub struct CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    user_query: Q,
    user_repository: R,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl<Q, R> CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    pub fn new(
        user_query: Q,
        user_repository: R,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            user_query,
            user_repository,
            password_hasher,
        }
    }

    // ========================================================================
    // Validation - Business Rules
    // ========================================================================

    fn validate_username(&self, username: &str) -> Result<String, CreateUserError> {
        let trimmed = username.trim();

        if trimmed.is_empty() {
            return Err(CreateUserError::InvalidUsername(
                "Username cannot be empty".to_string(),
            ));
        }

        if trimmed.len() < 3 || trimmed.len() > 50 {
            return Err(CreateUserError::InvalidUsername(
                "Username must be 3-50 characters".to_string(),
            ));
        }

        if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(CreateUserError::InvalidUsername(
                "Username can only contain letters, numbers, and underscores".to_string(),
            ));
        }

        // Return normalized (lowercase)
        Ok(trimmed.to_lowercase())
    }

    fn validate_email(&self, email: &str) -> Result<String, CreateUserError> {
        let trimmed = email.trim();

        if !EmailAddress::is_valid(trimmed) {
            return Err(CreateUserError::InvalidEmail(
                "Invalid email format".to_string(),
            ));
        }

        // Return normalized (lowercase)
        Ok(trimmed.to_lowercase())
    }

    fn validate_password(&self, password: &str) -> Result<(), CreateUserError> {
        if password.len() < 12 {
            return Err(CreateUserError::InvalidPassword(
                "Password must be at least 12 characters".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_full_name(&self, full_name: &str) -> Result<String, CreateUserError> {
        let trimmed = full_name.trim();

        if trimmed.is_empty() {
            return Err(CreateUserError::InvalidFullName(
                "Full name cannot be empty".to_string(),
            ));
        }

        if trimmed.len() > 100 {
            return Err(CreateUserError::InvalidFullName(
                "Full name cannot exceed 100 characters".to_string(),
            ));
        }

        // Return normalized
        Ok(trimmed.to_string())
    }

    // ========================================================================
    // Soft-Delete Check - Business Rule
    // ========================================================================

    async fn check_and_restore_soft_deleted(
        &self,
        email: &str,
    ) -> Result<Option<UserResult>, CreateUserError> {
        if let Some(existing_user) = self.user_query.find_by_email(email).await? {
            if existing_user.is_deleted {
                // Restore the user
                let restored = self.user_repository.restore_user(existing_user.id).await?;
                return Ok(Some(restored));
            }
        }
        Ok(None)
    }
}

// ============================================================================
// Use Case Execution - SINGLE RESPONSIBILITY: Create a User
// ============================================================================

#[async_trait]
impl<Q, R> ICreateUserUseCase for CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    async fn execute(&self, input: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
        // 1. Validate and normalize inputs
        let username = self.validate_username(&input.username)?;
        let email = self.validate_email(&input.email)?;
        self.validate_password(&input.password)?;
        let full_name = self.validate_full_name(&input.full_name)?;

        // 2. Check if user is soft-deleted and restore it
        //    Further more efficient and cleaner implementation needed to reduce the database hits
        if let Some(restored) = self.check_and_restore_soft_deleted(&email).await? {
            return Ok(CreateUserOutput {
                user_id: restored.id,
                email: restored.email,
                username: restored.username,
                full_name: restored.full_name,
            });
        }

        // 3. Hash password
        let password_hash = self
            .password_hasher
            .hash_password(&input.password)
            .await
            .map_err(|e| match e {
                HashError::HashFailed => {
                    CreateUserError::HashingFailed("password hashing failed".to_string())
                }
                HashError::VerifyFailed => {
                    CreateUserError::HashingFailed("unexpected verification failure".to_string())
                }
                HashError::TaskFailed => {
                    CreateUserError::HashingFailed("background task failed".to_string())
                }
            })?;

        // 4. Create user (database constraint catches duplicates)
        let created_user = self
            .user_repository
            .create_user(CreateUserData {
                email: email.clone(),
                username: username.clone(),
                password_hash,
                full_name: full_name.clone(),
            })
            .await
            .map_err(|e| match e {
                UserRepositoryError::UserAlreadyExists => CreateUserError::UserAlreadyExists,
                other => CreateUserError::RepositoryError(other),
            })?;

        // 5. Return created user
        // NOTE: Email verification is handled OUTSIDE this use case
        // (see application layer orchestration or domain events)
        Ok(CreateUserOutput {
            user_id: created_user.id,
            email: created_user.email,
            username: created_user.username,
            full_name: created_user.full_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::{
        password_hasher::{HashError, PasswordHasher},
        user_query::{UserQuery, UserQueryError, UserQueryResult},
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    // ======================================================================
    // Helpers
    // ======================================================================

    fn active_user_query(user: &UserResult) -> UserQueryResult {
        UserQueryResult {
            id: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            password_hash: "hashed".into(),
            full_name: user.full_name.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_verified: true,
            is_deleted: false,
        }
    }

    fn soft_deleted_user_query(user: &UserResult) -> UserQueryResult {
        UserQueryResult {
            is_deleted: true,
            ..active_user_query(user)
        }
    }

    fn valid_input() -> CreateUserInput {
        CreateUserInput {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password: "securepassword123".to_string(),
            full_name: "Test User".to_string(),
        }
    }

    // ======================================================================
    // Mock UserQuery
    // ======================================================================

    #[derive(Clone)]
    struct MockUserQuery {
        email_result: Option<UserQueryResult>,
    }

    impl MockUserQuery {
        fn empty() -> Self {
            Self { email_result: None }
        }

        fn with_active_user(user: UserResult) -> Self {
            Self {
                email_result: Some(active_user_query(&user)),
            }
        }

        fn with_soft_deleted_user(user: UserResult) -> Self {
            Self {
                email_result: Some(soft_deleted_user_query(&user)),
            }
        }
    }

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_email(&self, _: &str) -> Result<Option<UserQueryResult>, UserQueryError> {
            Ok(self.email_result.clone())
        }

        async fn find_by_username(
            &self,
            _: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            Ok(None)
        }

        async fn find_by_id(&self, _: Uuid) -> Result<Option<UserQueryResult>, UserQueryError> {
            Ok(None)
        }
    }

    // ======================================================================
    // Mock UserRepository
    // ======================================================================

    #[derive(Clone)]
    struct MockUserRepository {
        create_result: Option<Result<UserResult, UserRepositoryError>>,
        restore_result: Option<Result<UserResult, UserRepositoryError>>,
    }

    impl MockUserRepository {
        fn success_create(user: UserResult) -> Self {
            Self {
                create_result: Some(Ok(user)),
                restore_result: None,
            }
        }

        fn success_restore(user: UserResult) -> Self {
            Self {
                create_result: None,
                restore_result: Some(Ok(user)),
            }
        }

        fn create_error(err: UserRepositoryError) -> Self {
            Self {
                create_result: Some(Err(err)),
                restore_result: None,
            }
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn create_user(&self, _: CreateUserData) -> Result<UserResult, UserRepositoryError> {
            self.create_result
                .clone()
                .expect("create_user was not expected to be called")
        }

        async fn restore_user(&self, _: Uuid) -> Result<UserResult, UserRepositoryError> {
            self.restore_result
                .clone()
                .expect("restore_user was not expected to be called")
        }

        async fn activate_user(&self, _: Uuid) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }

        async fn set_full_name(
            &self,
            _: Uuid,
            _: String,
        ) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }

        async fn update_password(&self, _: Uuid, _: String) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }

        async fn delete_user(&self, _: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }

        async fn soft_delete_user(&self, _: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }
    }

    // ======================================================================
    // Mock PasswordHasher
    // ======================================================================

    #[derive(Clone)]
    struct MockPasswordHasher {
        result: Result<String, HashError>,
    }

    impl MockPasswordHasher {
        fn success() -> Self {
            Self {
                result: Ok("hashed_password".to_string()),
            }
        }

        fn fail() -> Self {
            Self {
                result: Err(HashError::HashFailed),
            }
        }
    }

    #[async_trait]
    impl PasswordHasher for MockPasswordHasher {
        async fn hash_password(&self, _: &str) -> Result<String, HashError> {
            self.result.clone()
        }

        async fn verify_password(&self, _: &str, _: &str) -> Result<bool, HashError> {
            Ok(true)
        }
    }

    // ======================================================================
    // TESTS — Soft Delete Restoration
    // ======================================================================

    #[tokio::test]
    async fn restores_soft_deleted_user_on_execute() {
        let deleted_user = UserResult {
            id: Uuid::new_v4(),
            email: "deleted@example.com".into(),
            username: "deleteduser".into(),
            full_name: "Deleted User".into(),
        };

        let restored_user = deleted_user.clone();

        let use_case = CreateUserUseCase::new(
            MockUserQuery::with_soft_deleted_user(deleted_user),
            MockUserRepository::success_restore(restored_user.clone()),
            Arc::new(MockPasswordHasher::success()),
        );

        let result = use_case.execute(valid_input()).await.unwrap();
        assert_eq!(result.user_id, restored_user.id);
        assert_eq!(result.email, restored_user.email);
    }

    #[tokio::test]
    async fn does_not_restore_active_user() {
        let active_user = UserResult {
            id: Uuid::new_v4(),
            email: "active@example.com".into(),
            username: "activeuser".into(),
            full_name: "Active User".into(),
        };

        let created_user = UserResult {
            id: Uuid::new_v4(),
            email: "test@example.com".into(),
            username: "testuser".into(),
            full_name: "Test User".into(),
        };

        let use_case = CreateUserUseCase::new(
            MockUserQuery::with_active_user(active_user),
            MockUserRepository::success_create(created_user.clone()),
            Arc::new(MockPasswordHasher::success()),
        );

        let result = use_case.execute(valid_input()).await.unwrap();
        assert_eq!(result.email, created_user.email);
    }

    // ======================================================================
    // TESTS — Errors
    // ======================================================================

    #[tokio::test]
    async fn fails_when_user_already_exists() {
        let use_case = CreateUserUseCase::new(
            MockUserQuery::empty(),
            MockUserRepository::create_error(UserRepositoryError::UserAlreadyExists),
            Arc::new(MockPasswordHasher::success()),
        );

        let err = use_case.execute(valid_input()).await.unwrap_err();
        assert!(matches!(err, CreateUserError::UserAlreadyExists));
    }

    #[tokio::test]
    async fn fails_when_hashing_fails() {
        let use_case = CreateUserUseCase::new(
            MockUserQuery::empty(),
            MockUserRepository::success_create(UserResult {
                id: Uuid::new_v4(),
                email: "x".into(),
                username: "x".into(),
                full_name: "x".into(),
            }),
            Arc::new(MockPasswordHasher::fail()),
        );

        let err = use_case.execute(valid_input()).await.unwrap_err();
        assert!(matches!(err, CreateUserError::HashingFailed(_)));
    }
}
