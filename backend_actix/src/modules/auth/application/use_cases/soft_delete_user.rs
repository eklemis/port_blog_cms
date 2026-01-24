use async_trait::async_trait;
use tracing::warn;
use uuid::Uuid;

use crate::auth::application::ports::outgoing::{
    token_repository::TokenRepository, UserRepository,
};

// ====================== Soft Delete Request ======================
#[derive(Debug, Clone)]
pub struct SoftDeleteUserRequest {
    user_id: Uuid,
}

impl SoftDeleteUserRequest {
    pub fn new(user_id: Uuid) -> Self {
        Self { user_id }
    }
}

// ====================== Soft Delete Errors =================
#[derive(Debug, Clone)]
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
}

impl<U, T> SoftDeleteUserUseCase<U, T>
where
    U: UserRepository + Send + Sync,
    T: TokenRepository + Send + Sync,
{
    pub fn new(user_repository: U, token_repository: T) -> Self {
        Self {
            user_repository,
            token_repository,
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
        let user_id = request.user_id;

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
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use crate::auth::application::ports::outgoing::{
        token_repository::TokenRepository,
        user_repository::{CreateUserData, UserRepository, UserRepositoryError, UserResult},
    };

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
        async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError> {
            if self.should_fail {
                return Err(UserRepositoryError::DatabaseError(
                    "user delete failed".to_string(),
                ));
            }

            self.deleted_users.lock().await.push(user_id);
            Ok(())
        }

        async fn create_user(
            &self,
            _data: CreateUserData,
        ) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
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
            unimplemented!()
        }
    }

    // ====================== Tests ======================

    #[tokio::test]
    async fn test_soft_delete_success_revokes_tokens_and_deletes_user() {
        let user_repo = MockUserRepository::new();
        let token_repo = MockTokenRepository::new();

        let use_case = SoftDeleteUserUseCase::new(user_repo.clone(), token_repo.clone());

        let user_id = Uuid::new_v4();
        let request = SoftDeleteUserRequest::new(user_id);

        let result = use_case.execute(request).await;

        assert!(result.is_ok());
        assert!(token_repo.was_revoked(user_id).await);
        assert!(user_repo.was_deleted(user_id).await);
    }

    #[tokio::test]
    async fn test_soft_delete_fails_if_token_revocation_fails() {
        let user_repo = MockUserRepository::new();
        let token_repo = MockTokenRepository::with_failure();

        let use_case = SoftDeleteUserUseCase::new(user_repo, token_repo);

        let request = SoftDeleteUserRequest::new(Uuid::new_v4());

        let result = use_case.execute(request).await;

        assert!(matches!(result, Err(SoftDeleteUserError::DatabaseError(_))));
    }

    #[tokio::test]
    async fn test_soft_delete_fails_if_user_delete_fails() {
        let user_repo = MockUserRepository::with_failure();
        let token_repo = MockTokenRepository::new();

        let use_case = SoftDeleteUserUseCase::new(user_repo, token_repo);

        let request = SoftDeleteUserRequest::new(Uuid::new_v4());

        let result = use_case.execute(request).await;

        assert!(matches!(result, Err(SoftDeleteUserError::DatabaseError(_))));
    }
}
