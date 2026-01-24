use std::sync::Arc;

use crate::auth::application::ports::outgoing::token_provider::{TokenError, TokenProvider};
use crate::modules::auth::application::ports::outgoing::{
    user_repository::UserRepository, UserRepositoryError,
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum VerifyUserEmailError {
    TokenExpired,
    TokenInvalid,
    UserNotFound,
    DatabaseError,
}

#[async_trait]
pub trait IVerifyUserEmailUseCase: Send + Sync {
    async fn execute(&self, token: &str) -> Result<(), VerifyUserEmailError>;
}

#[derive(Clone)]
pub struct VerifyUserEmailUseCase<R>
where
    R: UserRepository + Send + Sync,
{
    repository: R,
    token_provider: Arc<dyn TokenProvider>,
}

impl<R> VerifyUserEmailUseCase<R>
where
    R: UserRepository + Send + Sync,
{
    pub fn new(repository: R, token_provider: Arc<dyn TokenProvider>) -> Self {
        Self {
            repository,
            token_provider,
        }
    }
}

#[async_trait]
impl<R> IVerifyUserEmailUseCase for VerifyUserEmailUseCase<R>
where
    R: UserRepository + Send + Sync,
{
    async fn execute(&self, token: &str) -> Result<(), VerifyUserEmailError> {
        // Decode and validate token (logging handled in JWT service)
        let user_id = self
            .token_provider
            .verify_verification_token(token)
            .map_err(|e| match e {
                TokenError::TokenExpired => VerifyUserEmailError::TokenExpired,
                _ => VerifyUserEmailError::TokenInvalid,
            })?;

        // Activate user through user repository
        self.repository
            .activate_user(user_id)
            .await
            .map_err(|e| match e {
                UserRepositoryError::UserNotFound => VerifyUserEmailError::UserNotFound,
                UserRepositoryError::DatabaseError(_) => VerifyUserEmailError::DatabaseError,
                _ => VerifyUserEmailError::DatabaseError,
            })?;

        tracing::info!("User email verified successfully: {}", user_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenClaims;
    use crate::auth::application::ports::outgoing::user_repository::{CreateUserData, UserResult};
    use crate::modules::auth::application::ports::outgoing::{
        user_repository::UserRepository, UserRepositoryError,
    };
    use async_trait::async_trait;
    use mockall::{mock, predicate::*};
    use uuid::Uuid;

    // Mock UserRepository trait
    mock! {
        pub UserRepositoryMock {}
        #[async_trait]
        impl UserRepository for UserRepositoryMock {
            async fn create_user(&self, data: CreateUserData) -> Result<UserResult, UserRepositoryError>;
            async fn restore_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError>;
            async fn activate_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError>;
            async fn set_full_name(
                &self,
                user_id: Uuid,
                full_name: String,
            ) -> Result<UserResult, UserRepositoryError>;

            // Operations that don't need to return user data (pure commands)
            async fn update_password(
                &self,
                user_id: Uuid,
                new_password_hash: String,
            ) -> Result<(), UserRepositoryError>;
            async fn delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
            async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
        }
    }

    // Helper function to create JWT service
    fn create_jwt_service() -> JwtTokenService {
        let config = JwtConfig {
            secret_key: "testsecretkey_min_32_characters_long".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        };
        JwtTokenService::new(config)
    }

    #[tokio::test]
    async fn test_verify_user_email_success() {
        let mut repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();
        let jwt_service = create_jwt_service();

        let valid_token = jwt_service
            .generate_verification_token(user_id)
            .expect("Should generate verification token");

        // Set up expectations

        repository
            .expect_activate_user()
            .with(eq(user_id))
            .times(1)
            .returning(move |_| {
                Ok(UserResult {
                    id: user_id.to_owned(),
                    email: "test@example.com".to_string(),
                    username: "testuser".to_string(),
                    full_name: "Test User".to_string(),
                })
            });

        // Create use case with the configured repository
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(jwt_service));

        // Execute use case
        let result = use_case.execute(&valid_token).await;

        // Assert success
        assert!(
            result.is_ok(),
            "Expected successful execution, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_token_expired() {
        use chrono::{Duration, Utc};
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();

        let now = Utc::now();
        // Manually create an expired verification token (beyond leeway)
        let expired_claims = TokenClaims {
            sub: user_id,
            exp: (now - Duration::seconds(60)).timestamp(), // Expired 60 seconds ago (beyond 30s leeway)
            iat: (now - Duration::hours(25)).timestamp(),   // Issued 25 hours ago
            nbf: (now - Duration::hours(25)).timestamp(),   // Not before 25 hours ago
            token_type: "verification".to_string(),
            is_verified: false,
        };

        let expired_token = encode(
            &Header::new(Algorithm::HS256),
            &expired_claims,
            &EncodingKey::from_secret("testsecretkey_min_32_characters_long".as_bytes()),
        )
        .expect("Should encode expired token");

        // Create use case
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(create_jwt_service()));

        // Execute use case
        let result = use_case.execute(&expired_token).await;

        // Assert token expired error
        assert!(
            matches!(result, Err(VerifyUserEmailError::TokenExpired)),
            "Expected TokenExpired, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_token_invalid() {
        let repository = MockUserRepositoryMock::new();
        let invalid_token = "invalid.jwt.token";

        // Create use case (no repository expectations needed since token validation fails first)
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(create_jwt_service()));

        // Execute use case
        let result = use_case.execute(invalid_token).await;

        // Assert token invalid error
        assert!(
            matches!(result, Err(VerifyUserEmailError::TokenInvalid)),
            "Expected TokenInvalid, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_wrong_token_type() {
        let repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();
        let jwt_service = create_jwt_service();

        let access_token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Should generate access token");

        // Create use case (no repository expectations needed since token validation fails first)
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(jwt_service));

        // Execute use case
        let result = use_case.execute(&access_token).await;

        // Assert token invalid error (wrong token type)
        assert!(
            matches!(result, Err(VerifyUserEmailError::TokenInvalid)),
            "Expected TokenInvalid, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_with_refresh_token() {
        let repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();
        let jwt_service = create_jwt_service();

        let refresh_token = jwt_service
            .generate_refresh_token(user_id, true)
            .expect("Should generate refresh token");

        // Create use case
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(jwt_service));

        // Execute use case
        let result = use_case.execute(&refresh_token).await;

        // Assert token invalid error (wrong token type)
        assert!(
            matches!(result, Err(VerifyUserEmailError::TokenInvalid)),
            "Expected TokenInvalid for refresh token, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_with_invalid_signature() {
        let repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();

        // Create token with one secret
        let config1 = JwtConfig {
            secret_key: "first_secret_key_min_32_characters_long".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        };
        let jwt_service1 = JwtTokenService::new(config1);

        let token = jwt_service1
            .generate_verification_token(user_id)
            .expect("Should generate token");

        // Try to verify with different secret
        let jwt_service2 = create_jwt_service();
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(jwt_service2));

        // Execute use case
        let result = use_case.execute(&token).await;

        // Assert token invalid error (signature mismatch)
        assert!(
            matches!(result, Err(VerifyUserEmailError::TokenInvalid)),
            "Expected TokenInvalid for signature mismatch, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_with_malformed_token() {
        let repository = MockUserRepositoryMock::new();
        let malformed_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.bm90X3ZhbGlkX2pzb24.fakesig";

        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(create_jwt_service()));

        // Execute use case
        let result = use_case.execute(malformed_token).await;

        // Assert token invalid error
        assert!(
            matches!(result, Err(VerifyUserEmailError::TokenInvalid)),
            "Expected TokenInvalid for malformed token, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_user_not_found() {
        let mut repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();
        let jwt_service = create_jwt_service();

        let valid_token = jwt_service
            .generate_verification_token(user_id)
            .expect("Should generate verification token");

        // Set up expectations for repository to return user not found
        repository
            .expect_activate_user()
            .with(eq(user_id))
            .times(1)
            .returning(|_| Err(UserRepositoryError::UserNotFound));

        // Create use case with the configured repository
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(jwt_service));

        // Execute use case
        let result = use_case.execute(&valid_token).await;

        // Assert user not found error
        assert!(
            matches!(result, Err(VerifyUserEmailError::UserNotFound)),
            "Expected UserNotFound, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_database_error() {
        let mut repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();
        let jwt_service = create_jwt_service();

        let valid_token = jwt_service
            .generate_verification_token(user_id)
            .expect("Should generate verification token");

        // Set up expectations for repository to return database error
        repository
            .expect_activate_user()
            .with(eq(user_id))
            .times(1)
            .returning(|_| Err(UserRepositoryError::DatabaseError("DB error".to_string())));

        // Create use case with the configured repository
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(jwt_service));

        // Execute use case
        let result = use_case.execute(&valid_token).await;

        // Assert database error
        assert!(
            matches!(result, Err(VerifyUserEmailError::DatabaseError)),
            "Expected DatabaseError, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_repository_unknown_error() {
        let mut repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();
        let jwt_service = create_jwt_service();

        let valid_token = jwt_service
            .generate_verification_token(user_id)
            .expect("Should generate verification token");

        // Set up expectations for repository to return an unexpected error variant
        repository
            .expect_activate_user()
            .with(eq(user_id))
            .times(1)
            .returning(|_| Err(UserRepositoryError::UserAlreadyExists));

        // Create use case with the configured repository
        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(jwt_service));

        // Execute use case
        let result = use_case.execute(&valid_token).await;

        // Assert database error (catch-all maps to DatabaseError)
        assert!(
            matches!(result, Err(VerifyUserEmailError::DatabaseError)),
            "Expected DatabaseError, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_user_email_with_tampered_token() {
        let repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();
        let jwt_service = create_jwt_service();

        let mut valid_token = jwt_service
            .generate_verification_token(user_id)
            .expect("Should generate verification token");

        // Tamper with the token
        valid_token.push('x');

        let use_case = VerifyUserEmailUseCase::new(repository, Arc::new(jwt_service));

        // Execute use case
        let result = use_case.execute(&valid_token).await;

        // Assert token invalid error
        assert!(
            matches!(result, Err(VerifyUserEmailError::TokenInvalid)),
            "Expected TokenInvalid for tampered token, got {:?}",
            result
        );
    }
}
