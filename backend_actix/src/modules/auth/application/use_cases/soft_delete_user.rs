use async_trait::async_trait;
use tracing::warn;

use crate::auth::application::{
    ports::outgoing::{token_repository::TokenRepository, UserRepository},
    services::jwt::JwtService,
};

// ====================== Soft Delete Request ======================
#[derive(Debug, Clone)]
pub struct SoftDeleteUserRequest {
    access_token: String,
}

impl SoftDeleteUserRequest {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token: access_token.trim().to_string(),
        }
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }
}

// ====================== Soft Delete Errors =================
#[derive(Debug)]
pub enum SoftDeleteUserError {
    Unauthorized,
    DatabaseError(String),
}

impl std::fmt::Display for SoftDeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftDeleteUserError::Unauthorized => write!(f, "Unauthorized"),
            SoftDeleteUserError::DatabaseError(msg) => {
                write!(f, "Database error: {}", msg)
            }
        }
    }
}

impl std::error::Error for SoftDeleteUserError {}

// ==================== Soft Delete Use Case ======================
#[async_trait]
pub trait ISoftDeleteUserUseCase: Send + Sync {
    async fn execute(&self, request: SoftDeleteUserRequest) -> Result<(), SoftDeleteUserError>;
}

pub struct SoftDeleteUserUseCase<U, T>
where
    U: UserRepository + Send + Sync,
    T: TokenRepository + Send + Sync,
{
    user_repository: U,
    token_repository: T,
    jwt_service: JwtService,
}

impl<U, T> SoftDeleteUserUseCase<U, T>
where
    U: UserRepository + Send + Sync,
    T: TokenRepository + Send + Sync,
{
    pub fn new(user_repository: U, token_repository: T, jwt_service: JwtService) -> Self {
        Self {
            user_repository,
            token_repository,
            jwt_service,
        }
    }
}

#[async_trait]
impl<U, T> ISoftDeleteUserUseCase for SoftDeleteUserUseCase<U, T>
where
    U: UserRepository + Send + Sync,
    T: TokenRepository + Send + Sync,
{
    async fn execute(&self, request: SoftDeleteUserRequest) -> Result<(), SoftDeleteUserError> {
        // 🔐 Single source of truth
        let claims = self
            .jwt_service
            .verify_token(request.access_token())
            .map_err(|_| SoftDeleteUserError::Unauthorized)?;

        let user_id = claims.sub;

        // 🔥 Revoke all tokens
        self.token_repository
            .revoke_all_user_tokens(user_id)
            .await
            .map_err(|e| SoftDeleteUserError::DatabaseError(e.to_string()))?;

        // 🗑 Soft delete user
        self.user_repository
            .soft_delete_user(user_id)
            .await
            .map_err(|e| SoftDeleteUserError::DatabaseError(e.to_string()))?;

        warn!("User {} soft deleted", user_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::{
        domain::entities::User,
        ports::outgoing::UserRepositoryError,
        services::jwt::{JwtConfig, JwtService},
    };
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    // ====================== Mock Token Repository ======================
    #[derive(Default, Clone)]
    struct MockTokenRepository {
        revoked_users: Arc<Mutex<Vec<Uuid>>>,
        should_fail: bool,
    }

    impl MockTokenRepository {
        fn new() -> Self {
            Self::default()
        }

        fn with_failure() -> Self {
            Self {
                revoked_users: Arc::new(Mutex::new(Vec::new())),
                should_fail: true,
            }
        }

        async fn was_revoked(&self, user_id: Uuid) -> bool {
            self.revoked_users.lock().await.contains(&user_id)
        }
    }

    #[async_trait]
    impl TokenRepository for MockTokenRepository {
        async fn blacklist_token(
            &self,
            _token_hash: String,
            _user_id: Uuid,
            _expires_at: DateTime<Utc>,
        ) -> Result<
            (),
            crate::auth::application::ports::outgoing::token_repository::TokenRepositoryError,
        > {
            Ok(())
        }

        async fn is_token_blacklisted(
            &self,
            _token_hash: &str,
        ) -> Result<
            bool,
            crate::auth::application::ports::outgoing::token_repository::TokenRepositoryError,
        > {
            Ok(false)
        }

        async fn remove_blacklisted_token(
            &self,
            _token_hash: &str,
        ) -> Result<
            (),
            crate::auth::application::ports::outgoing::token_repository::TokenRepositoryError,
        > {
            Ok(())
        }

        async fn revoke_all_user_tokens(
            &self,
            user_id: Uuid,
        ) -> Result<
            (),
            crate::auth::application::ports::outgoing::token_repository::TokenRepositoryError,
        > {
            if self.should_fail {
                return Err(
                    crate::auth::application::ports::outgoing::token_repository::TokenRepositoryError::DatabaseError(
                        "token revoke failed".to_string(),
                    ),
                );
            }

            self.revoked_users.lock().await.push(user_id);
            Ok(())
        }

        async fn cleanup_expired_tokens(
            &self,
        ) -> Result<
            u64,
            crate::auth::application::ports::outgoing::token_repository::TokenRepositoryError,
        > {
            Ok(0)
        }
    }

    // ====================== Mock User Repository ======================
    #[derive(Default, Clone)]
    struct MockUserRepository {
        deleted_users: Arc<Mutex<Vec<Uuid>>>,
        should_fail: bool,
    }

    impl MockUserRepository {
        fn new() -> Self {
            Self::default()
        }

        fn with_failure() -> Self {
            Self {
                deleted_users: Arc::new(Mutex::new(Vec::new())),
                should_fail: true,
            }
        }

        async fn was_deleted(&self, user_id: Uuid) -> bool {
            self.deleted_users.lock().await.contains(&user_id)
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn create_user(&self, _user: User) -> Result<User, UserRepositoryError> {
            unimplemented!()
        }
        async fn soft_delete_user(
            &self,
            user_id: Uuid,
        ) -> Result<
            (),
            crate::auth::application::ports::outgoing::user_repository::UserRepositoryError,
        > {
            if self.should_fail {
                return Err(
                    crate::auth::application::ports::outgoing::user_repository::UserRepositoryError::DatabaseError(
                        "user delete failed".to_string(),
                    ),
                );
            }

            self.deleted_users.lock().await.push(user_id);
            Ok(())
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
        async fn restore_user(&self, _user_id: Uuid) -> Result<User, UserRepositoryError> {
            unimplemented!()
        }
        async fn activate_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }
    }

    // ====================== Helpers ======================
    fn create_jwt_service() -> JwtService {
        JwtService::new(JwtConfig {
            secret_key: "test_secret_key_min_32_characters_long".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        })
    }

    // ====================== Tests ======================

    #[tokio::test]
    async fn test_soft_delete_success_revokes_tokens_and_deletes_user() {
        let user_repo = MockUserRepository::new();
        let token_repo = MockTokenRepository::new();
        let jwt_service = create_jwt_service();

        let user_id = Uuid::new_v4();
        let access_token = jwt_service.generate_access_token(user_id, true).unwrap();

        let use_case =
            SoftDeleteUserUseCase::new(user_repo.clone(), token_repo.clone(), jwt_service);

        let request = SoftDeleteUserRequest::new(access_token);

        let result = use_case.execute(request).await;

        assert!(result.is_ok());
        assert!(token_repo.was_revoked(user_id).await);
        assert!(user_repo.was_deleted(user_id).await);
    }

    #[tokio::test]
    async fn test_soft_delete_with_invalid_token_returns_unauthorized() {
        let user_repo = MockUserRepository::new();
        let token_repo = MockTokenRepository::new();
        let jwt_service = create_jwt_service();

        let use_case = SoftDeleteUserUseCase::new(user_repo, token_repo, jwt_service);

        let request = SoftDeleteUserRequest::new("invalid.jwt.token".to_string());

        let result = use_case.execute(request).await;

        assert!(matches!(result, Err(SoftDeleteUserError::Unauthorized)));
    }

    #[tokio::test]
    async fn test_soft_delete_fails_if_token_revocation_fails() {
        let user_repo = MockUserRepository::new();
        let token_repo = MockTokenRepository::with_failure();
        let jwt_service = create_jwt_service();

        let user_id = Uuid::new_v4();
        let access_token = jwt_service.generate_access_token(user_id, true).unwrap();

        let use_case = SoftDeleteUserUseCase::new(user_repo, token_repo, jwt_service);

        let request = SoftDeleteUserRequest::new(access_token);

        let result = use_case.execute(request).await;

        assert!(matches!(result, Err(SoftDeleteUserError::DatabaseError(_))));
    }

    #[tokio::test]
    async fn test_soft_delete_fails_if_user_delete_fails() {
        let user_repo = MockUserRepository::with_failure();
        let token_repo = MockTokenRepository::new();
        let jwt_service = create_jwt_service();

        let user_id = Uuid::new_v4();
        let access_token = jwt_service.generate_access_token(user_id, true).unwrap();

        let use_case = SoftDeleteUserUseCase::new(user_repo, token_repo, jwt_service);

        let request = SoftDeleteUserRequest::new(access_token);

        let result = use_case.execute(request).await;

        assert!(matches!(result, Err(SoftDeleteUserError::DatabaseError(_))));
    }
}
