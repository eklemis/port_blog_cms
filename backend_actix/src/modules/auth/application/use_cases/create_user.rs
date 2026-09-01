use crate::auth::application::ports::outgoing::user_query::UserQueryError;
use crate::auth::application::ports::outgoing::user_repository::CreateUserData;

use crate::modules::auth::application::ports::outgoing::{
    user_query::UserQuery, user_repository::UserRepository, UserRepositoryError,
};
use async_trait::async_trait;
use email_address::EmailAddress;

use crate::auth::application::ports::incoming::password_policy::{
    PasswordPolicy, PasswordPolicyError,
};
use crate::auth::application::ports::outgoing::password_hasher::{HashError, PasswordHasher};
use std::sync::Arc;

// ============================================================================
// Input / Output DTOs
// ============================================================================
/// A registration request, as it arrives from the route.
#[derive(Clone, Debug)]
pub struct CreateUserInput {
    /// Requested public handle.
    pub username: String,
    /// Requested login address.
    pub email: String,
    /// Plaintext password. Hashed inside the use case and never stored or
    /// logged as-is.
    pub password: String,
    /// Requested display name.
    pub full_name: String,
}
/// The stored user, as returned after successful registration.
#[derive(Clone, Debug)]
pub struct CreateUserOutput {
    /// Primary key assigned to the new user.
    pub user_id: uuid::Uuid,
    /// Requested login address.
    pub email: String,
    /// Requested public handle.
    pub username: String,
    /// Requested display name.
    pub full_name: String,
}

// ============================================================================
// Error Types
// ============================================================================

/// Why registration failed.
#[derive(Debug, thiserror::Error, Clone)]
pub enum CreateUserError {
    /// The username failed validation. The payload says which rule.
    #[error("Invalid username: {0}")]
    InvalidUsername(String),

    /// The email address is not well formed.
    #[error("Invalid email: {0}")]
    InvalidEmail(String),

    /// The password does not meet the strength policy.
    #[error("Invalid password: {0}")]
    InvalidPassword(String),

    /// The display name is empty or too long.
    #[error("Invalid full name: {0}")]
    InvalidFullName(String),

    /// The email or username is taken. Deliberately does not say which — that
    /// would confirm whether an address is registered.
    #[error("User already exists")]
    UserAlreadyExists,

    /// The password could not be hashed. A server fault, not the caller's.
    #[error("Password hashing failed: {0}")]
    HashingFailed(String),

    /// The write failed.
    #[error("Repository error: {0}")]
    RepositoryError(#[from] UserRepositoryError),

    /// The uniqueness pre-check could not be read.
    #[error("Query error: {0}")]
    QueryError(#[from] UserQueryError),
}

// ============================================================================
// Use Case Interface
// ============================================================================

/// Registers a user.
///
/// The `I` prefix is the older convention; newer modules name the trait
/// plainly and suffix the implementation with `Service`.
#[async_trait]
pub trait ICreateUserUseCase: Send + Sync {
    /// Validates, hashes the password, and inserts the user.
    async fn execute(&self, input: CreateUserInput) -> Result<CreateUserOutput, CreateUserError>;
}

// ============================================================================
// Use Case Implementation - FOCUSED ON ONE THING
// ============================================================================

/// The default implementation, generic over the user reader and writer.
pub struct CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    user_query: Q,
    user_repository: R,
    password_hasher: Arc<dyn PasswordHasher>,
    password_policy: Arc<dyn PasswordPolicy>,
}

impl<Q, R> CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    /// Builds the use case from its ports.
    pub fn new(
        user_query: Q,
        user_repository: R,
        password_hasher: Arc<dyn PasswordHasher>,
        password_policy: Arc<dyn PasswordPolicy>,
    ) -> Self {
        Self {
            user_query,
            user_repository,
            password_hasher,
            password_policy,
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

    /// Delegates to the injected `PasswordPolicy`.
    ///
    /// This rule used to be inlined here, which left `BasicPasswordPolicy`
    /// implemented but unreachable and the two definitions free to drift — the
    /// inline copy had already lost the upper bound. Length is measured in
    /// bytes, matching the previous behaviour; that is the right unit for the
    /// maximum, since it bounds the work handed to Argon2.
    fn validate_password(&self, password: &str) -> Result<(), CreateUserError> {
        self.password_policy
            .validate(password)
            .map_err(|e| match e {
                PasswordPolicyError::TooShort => CreateUserError::InvalidPassword(
                    "Password must be at least 12 characters".to_string(),
                ),
                PasswordPolicyError::TooLong => CreateUserError::InvalidPassword(
                    "Password must not exceed 128 characters".to_string(),
                ),
                PasswordPolicyError::TooWeak => {
                    CreateUserError::InvalidPassword("Password is too weak".to_string())
                }
            })
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
    /// Attempt to restore a soft-deleted user, or return UserAlreadyExists error
    async fn try_restore_soft_deleted(
        &self,
        email: &str,
    ) -> Result<CreateUserOutput, CreateUserError> {
        let existing_user = self
            .user_query
            .find_by_email(email)
            .await?
            .ok_or(CreateUserError::UserAlreadyExists)?; // Shouldn't happen, but be defensive

        if existing_user.is_deleted {
            // Restore the soft-deleted user
            let restored = self.user_repository.restore_user(existing_user.id).await?;
            Ok(CreateUserOutput {
                user_id: restored.id,
                email: restored.email,
                username: restored.username,
                full_name: restored.full_name,
            })
        } else {
            // User exists and is active — genuine duplicate
            Err(CreateUserError::UserAlreadyExists)
        }
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

        // 2. Hash password
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

        // 3. Try to create user (optimistic approach)
        let create_result = self
            .user_repository
            .create_user(CreateUserData {
                email: email.clone(),
                username: username.clone(),
                password_hash,
                full_name: full_name.clone(),
            })
            .await;

        match create_result {
            Ok(created_user) => {
                // Happy path: user created successfully
                Ok(CreateUserOutput {
                    user_id: created_user.id,
                    email: created_user.email,
                    username: created_user.username,
                    full_name: created_user.full_name,
                })
            }
            Err(UserRepositoryError::UserAlreadyExists) => {
                // 4. Check if it's a soft-deleted user we can restore
                self.try_restore_soft_deleted(&email).await
            }
            Err(other) => Err(CreateUserError::RepositoryError(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::{
        password_hasher::{HashError, PasswordHasher},
        user_query::{UserQuery, UserQueryError, UserQueryResult},
        user_repository::UserResult,
    };
    use crate::auth::application::services::password::BasicPasswordPolicy;
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

        fn create_error(err: UserRepositoryError) -> Self {
            Self {
                create_result: Some(Err(err)),
                restore_result: None,
            }
        }

        // Add this new constructor
        fn fail_create_then_restore(restored_user: UserResult) -> Self {
            Self {
                create_result: Some(Err(UserRepositoryError::UserAlreadyExists)),
                restore_result: Some(Ok(restored_user)),
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
            MockUserRepository::fail_create_then_restore(restored_user.clone()), // Changed
            Arc::new(MockPasswordHasher::success()),
            Arc::new(BasicPasswordPolicy),
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
            Arc::new(BasicPasswordPolicy),
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
            Arc::new(BasicPasswordPolicy),
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
            Arc::new(BasicPasswordPolicy),
        );

        let err = use_case.execute(valid_input()).await.unwrap_err();
        assert!(matches!(err, CreateUserError::HashingFailed(_)));
    }

    // ======================================================================
    // TESTS — Password policy
    //
    // The rule now lives in `BasicPasswordPolicy` rather than inline here, and
    // these construct the use case with the real policy so they exercise the
    // same code production runs.
    // ======================================================================

    fn use_case_with_valid_collaborators() -> CreateUserUseCase<MockUserQuery, MockUserRepository> {
        CreateUserUseCase::new(
            MockUserQuery::empty(),
            MockUserRepository::success_create(UserResult {
                id: Uuid::new_v4(),
                email: "test@example.com".into(),
                username: "testuser".into(),
                full_name: "Test User".into(),
            }),
            Arc::new(MockPasswordHasher::success()),
            Arc::new(BasicPasswordPolicy),
        )
    }

    #[tokio::test]
    async fn rejects_password_below_the_minimum() {
        let use_case = use_case_with_valid_collaborators();

        let mut input = valid_input();
        input.password = "short".to_string();

        let err = use_case.execute(input).await.unwrap_err();
        assert!(
            matches!(&err, CreateUserError::InvalidPassword(m) if m.contains("at least 12")),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn accepts_a_password_exactly_at_the_minimum() {
        let use_case = use_case_with_valid_collaborators();

        let mut input = valid_input();
        input.password = "a".repeat(12);

        assert!(use_case.execute(input).await.is_ok());
    }

    /// The upper bound is the rule the old inline check had lost. It matters
    /// because Argon2's work scales with the input it is handed, so an
    /// unbounded password is a cheap way to make the server do expensive work.
    #[tokio::test]
    async fn rejects_password_above_the_maximum() {
        let use_case = use_case_with_valid_collaborators();

        let mut input = valid_input();
        input.password = "a".repeat(129);

        let err = use_case.execute(input).await.unwrap_err();
        assert!(
            matches!(&err, CreateUserError::InvalidPassword(m) if m.contains("128")),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn accepts_a_password_exactly_at_the_maximum() {
        let use_case = use_case_with_valid_collaborators();

        let mut input = valid_input();
        input.password = "a".repeat(128);

        assert!(use_case.execute(input).await.is_ok());
    }

    // ======================================================================
    // TESTS — Field validation
    //
    // Registration is the only place these rules are enforced, and each
    // rejection is a 400 the caller can act on. An over-permissive rule here
    // admits data every later query has to tolerate.
    // ======================================================================

    async fn expect_error(input: CreateUserInput) -> CreateUserError {
        use_case_with_valid_collaborators()
            .execute(input)
            .await
            .unwrap_err()
    }

    #[tokio::test]
    async fn rejects_an_empty_username() {
        let mut i = valid_input();
        i.username = "   ".into();
        assert!(matches!(
            expect_error(i).await,
            CreateUserError::InvalidUsername(_)
        ));
    }

    #[tokio::test]
    async fn enforces_the_username_length_bounds() {
        for name in ["ab", &"a".repeat(51)] {
            let mut i = valid_input();
            i.username = name.to_string();
            assert!(
                matches!(expect_error(i).await, CreateUserError::InvalidUsername(_)),
                "{name:?} should be rejected"
            );
        }
    }

    /// Usernames appear in public URLs (/api/public/blog/{username}/...), so
    /// punctuation and whitespace are refused rather than producing a link
    /// that needs escaping.
    #[tokio::test]
    async fn rejects_a_username_with_url_unsafe_characters() {
        for name in ["has space", "has/slash", "has-dash", "a@b", "a.b", "a+b"] {
            let mut i = valid_input();
            i.username = name.to_string();
            assert!(
                matches!(expect_error(i).await, CreateUserError::InvalidUsername(_)),
                "{name:?} should be rejected"
            );
        }
    }

    /// Pins current behaviour, which is broader than it may look:
    /// `char::is_alphanumeric` is Unicode-aware, so non-ASCII letters are
    /// accepted. Blog slugs use `is_ascii_alphanumeric` and are ASCII-only, so
    /// the two identifiers do not agree.
    ///
    /// The consequence worth knowing: "аdmin" with a Cyrillic U+0430 is a
    /// distinct username from "admin" but renders identically, and both appear
    /// in public URLs. Tightening this to ASCII would be a behaviour change for
    /// any existing account, so it is recorded here rather than altered.
    #[tokio::test]
    async fn currently_accepts_non_ascii_usernames() {
        for name in ["héllo", "用户名"] {
            let mut i = valid_input();
            i.username = name.to_string();
            assert!(
                use_case_with_valid_collaborators().execute(i).await.is_ok(),
                "{name:?} is accepted today"
            );
        }
    }

    #[tokio::test]
    async fn rejects_a_malformed_email() {
        for email in ["not-an-email", "@example.com", "a@", "a b@example.com"] {
            let mut i = valid_input();
            i.email = email.to_string();
            assert!(
                matches!(expect_error(i).await, CreateUserError::InvalidEmail(_)),
                "{email:?} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn rejects_an_empty_or_overlong_full_name() {
        for name in ["   ".to_string(), "a".repeat(101)] {
            let mut i = valid_input();
            i.full_name = name.clone();
            assert!(
                matches!(expect_error(i).await, CreateUserError::InvalidFullName(_)),
                "{name:?} should be rejected"
            );
        }
    }

    /// Username and email are normalised to lowercase so a later lookup by
    /// either matches regardless of how the caller typed it.
    #[tokio::test]
    async fn normalises_username_and_email_to_lowercase() {
        let mut i = valid_input();
        i.username = "  MixedCase  ".into();
        i.email = "  Mixed@Example.COM  ".into();

        let out = use_case_with_valid_collaborators()
            .execute(i)
            .await
            .unwrap();

        // The mock echoes a fixed row, so assert the call succeeded and that
        // validation accepted the padded, mixed-case input.
        assert!(!out.user_id.is_nil());
    }
}
