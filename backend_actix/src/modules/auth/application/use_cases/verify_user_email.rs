use crate::auth::application::services::jwt::JwtService;
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

#[derive(Debug, Clone)]
pub struct VerifyUserEmailUseCase<R>
where
    R: UserRepository + Send + Sync,
{
    repository: R,
    jwt_service: JwtService,
}

impl<R> VerifyUserEmailUseCase<R>
where
    R: UserRepository + Send + Sync,
{
    pub fn new(repository: R, jwt_service: JwtService) -> Self {
        Self {
            repository,
            jwt_service,
        }
    }
}

#[async_trait]
impl<R> IVerifyUserEmailUseCase for VerifyUserEmailUseCase<R>
where
    R: UserRepository + Send + Sync,
{
    async fn execute(&self, token: &str) -> Result<(), VerifyUserEmailError> {
        // Decode and validate token
        let user_id = self
            .jwt_service
            .verify_verification_token(token)
            .map_err(|e| match e.as_str() {
                "TokenExpired" => VerifyUserEmailError::TokenExpired,
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::services::jwt::{JwtConfig, JwtService};
    use crate::modules::auth::application::domain::entities::User;
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
                async fn create_user(&self, user: User) -> Result<User, UserRepositoryError>;

                async fn update_password(
                    &self,
                    user_id: Uuid,
                    new_password_hash: String,
                ) -> Result<(), UserRepositoryError>;

                async fn delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
                async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
                async fn restore_user(&self, user_id: Uuid) -> Result<User, UserRepositoryError>;
                async fn activate_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
        }
    }

    // Helper function to create JWT service
    fn create_jwt_service() -> JwtService {
        let config = JwtConfig {
            secret_key: "testsecretkey".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
        };
        JwtService::new(config)
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
            .returning(|_| Ok(()));

        // Create use case with the configured repository
        let use_case = VerifyUserEmailUseCase::new(repository, jwt_service);

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
        use crate::auth::application::services::jwt::JwtClaims;
        use chrono::{Duration, Utc};
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let repository = MockUserRepositoryMock::new();
        let user_id = Uuid::new_v4();

        // Manually create an expired verification token
        let expired_claims = JwtClaims {
            sub: user_id,
            exp: (Utc::now() - Duration::seconds(10)).timestamp(), // Expired 10 seconds ago
            token_type: "verification".to_string(),
        };

        let expired_token = encode(
            &Header::new(Algorithm::HS256),
            &expired_claims,
            &EncodingKey::from_secret("testsecretkey".as_bytes()),
        )
        .expect("Should encode expired token");

        // Create use case
        let use_case = VerifyUserEmailUseCase::new(repository, create_jwt_service());

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
        let use_case = VerifyUserEmailUseCase::new(repository, create_jwt_service());

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
            .generate_access_token(user_id)
            .expect("Should generate access token");

        // Create use case (no repository expectations needed since token validation fails first)
        let use_case = VerifyUserEmailUseCase::new(repository, jwt_service);

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
        let use_case = VerifyUserEmailUseCase::new(repository, jwt_service);

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
        let use_case = VerifyUserEmailUseCase::new(repository, jwt_service);

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
        let use_case = VerifyUserEmailUseCase::new(repository, jwt_service);

        // Execute use case
        let result = use_case.execute(&valid_token).await;

        // Assert database error (catch-all maps to DatabaseError)
        assert!(
            matches!(result, Err(VerifyUserEmailError::DatabaseError)),
            "Expected DatabaseError, got {:?}",
            result
        );
    }
}
