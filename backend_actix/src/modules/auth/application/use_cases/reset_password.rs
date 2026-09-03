use async_trait::async_trait;
use std::sync::Arc;

use crate::auth::application::ports::incoming::password_policy::{
    PasswordPolicy, PasswordPolicyError,
};
use crate::auth::application::ports::outgoing::password_hasher::PasswordHasher;
use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
use crate::auth::application::ports::outgoing::token_repository::TokenRepository;
use crate::auth::application::ports::outgoing::user_repository::UserRepository;

/// Why a reset could not be completed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResetPasswordError {
    /// The reset token is malformed, expired, or of the wrong kind.
    #[error("Invalid or expired reset token")]
    InvalidToken,

    /// The new password does not meet the strength policy.
    #[error("Invalid password: {0}")]
    InvalidPassword(String),

    /// The token was valid but names a user who no longer exists.
    #[error("User not found")]
    UserNotFound,

    /// The new password could not be hashed.
    #[error("Password hashing failed: {0}")]
    HashingFailed(String),

    /// The write failed.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

/// Completes a password reset by redeeming a token.
#[async_trait]
pub trait IResetPasswordUseCase: Send + Sync {
    /// Redeems the token and replaces the password.
    async fn execute(&self, token: &str, new_password: &str) -> Result<(), ResetPasswordError>;
}

/// The default implementation, generic over the user writer and the token
/// provider.
pub struct ResetPasswordUseCase<R, T>
where
    R: UserRepository + Send + Sync,
    T: TokenRepository + Send + Sync,
{
    user_repository: R,
    token_repository: T,
    token_provider: Arc<dyn TokenProvider + Send + Sync>,
    password_hasher: Arc<dyn PasswordHasher>,
    password_policy: Arc<dyn PasswordPolicy>,
}

impl<R, T> ResetPasswordUseCase<R, T>
where
    R: UserRepository + Send + Sync,
    T: TokenRepository + Send + Sync,
{
    /// Builds the use case from its ports.
    pub fn new(
        user_repository: R,
        token_repository: T,
        token_provider: Arc<dyn TokenProvider + Send + Sync>,
        password_hasher: Arc<dyn PasswordHasher>,
        password_policy: Arc<dyn PasswordPolicy>,
    ) -> Self {
        Self {
            user_repository,
            token_repository,
            token_provider,
            password_hasher,
            password_policy,
        }
    }
}

#[async_trait]
impl<R, T> IResetPasswordUseCase for ResetPasswordUseCase<R, T>
where
    R: UserRepository + Send + Sync,
    T: TokenRepository + Send + Sync,
{
    async fn execute(&self, token: &str, new_password: &str) -> Result<(), ResetPasswordError> {
        // Only a token minted for reset is accepted; a verification or access
        // token fails here on its token_type.
        let user_id = self
            .token_provider
            .verify_password_reset_token(token)
            .map_err(|_| ResetPasswordError::InvalidToken)?;

        // Same policy registration enforces, so a reset cannot be used to slip
        // past the length bounds.
        self.password_policy
            .validate(new_password)
            .map_err(|e| match e {
                PasswordPolicyError::TooShort => ResetPasswordError::InvalidPassword(
                    "Password must be at least 12 characters".to_string(),
                ),
                PasswordPolicyError::TooLong => ResetPasswordError::InvalidPassword(
                    "Password must not exceed 128 characters".to_string(),
                ),
                PasswordPolicyError::TooWeak => {
                    ResetPasswordError::InvalidPassword("Password is too weak".to_string())
                }
            })?;

        let hash = self
            .password_hasher
            .hash_password(new_password)
            .await
            .map_err(|e| ResetPasswordError::HashingFailed(e.to_string()))?;

        self.user_repository
            .update_password(user_id, hash)
            .await
            .map_err(|e| ResetPasswordError::RepositoryError(e.to_string()))?;

        // A reset is the remedy for a compromised account, so every existing
        // session must die with the old password. Failing to revoke would leave
        // an attacker's refresh token working after the owner "fixed" things.
        if let Err(e) = self.token_repository.revoke_all_user_tokens(user_id).await {
            tracing::error!(
                "Password reset for {} succeeded but session revocation failed: {}",
                user_id,
                e
            );
            return Err(ResetPasswordError::RepositoryError(format!(
                "password updated but sessions could not be revoked: {e}"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::password_hasher::HashError;
    use crate::auth::application::ports::outgoing::token_provider::{TokenClaims, TokenError};
    use crate::auth::application::ports::outgoing::token_repository::TokenRepositoryError;
    use crate::auth::application::ports::outgoing::user_repository::{
        CreateUserData, UserRepositoryError, UserResult,
    };
    use crate::auth::application::services::password::BasicPasswordPolicy;
    use chrono::{DateTime, Utc};
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    struct StubTokens {
        result: Result<Uuid, ()>,
    }

    impl TokenProvider for StubTokens {
        fn generate_access_token(&self, _u: Uuid, _v: bool) -> Result<String, TokenError> {
            unimplemented!()
        }
        fn generate_refresh_token(&self, _u: Uuid, _v: bool) -> Result<String, TokenError> {
            unimplemented!()
        }
        fn verify_token(&self, _t: &str) -> Result<TokenClaims, TokenError> {
            unimplemented!()
        }
        fn refresh_access_token(&self, _t: &str) -> Result<String, TokenError> {
            unimplemented!()
        }
        fn generate_verification_token(&self, _u: Uuid) -> Result<String, TokenError> {
            unimplemented!()
        }
        fn verify_verification_token(&self, _t: &str) -> Result<Uuid, TokenError> {
            unimplemented!()
        }
        fn generate_password_reset_token(&self, _u: Uuid) -> Result<String, TokenError> {
            unimplemented!()
        }
        fn verify_password_reset_token(&self, _t: &str) -> Result<Uuid, TokenError> {
            self.result
                .map_err(|_| TokenError::InvalidTokenType("password_reset".to_string()))
        }
    }

    #[derive(Default)]
    struct SpyRepo {
        updated: Mutex<Option<(Uuid, String)>>,
    }

    #[async_trait]
    impl UserRepository for SpyRepo {
        async fn set_profile(
            &self,
            user_id: Uuid,
            full_name: String,
            _bio: Option<Option<String>>,
            _locale: Option<String>,
        ) -> Result<UserResult, UserRepositoryError> {
            self.set_full_name(user_id, full_name).await
        }

        async fn create_user(&self, _d: CreateUserData) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }
        async fn restore_user(&self, _u: Uuid) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }
        async fn activate_user(&self, _u: Uuid) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }
        async fn set_full_name(
            &self,
            _u: Uuid,
            _n: String,
        ) -> Result<UserResult, UserRepositoryError> {
            unimplemented!()
        }
        async fn update_password(
            &self,
            user_id: Uuid,
            hash: String,
        ) -> Result<(), UserRepositoryError> {
            *self.updated.lock().unwrap() = Some((user_id, hash));
            Ok(())
        }
        async fn delete_user(&self, _u: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }
        async fn soft_delete_user(&self, _u: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct SpyTokenRepo {
        revoked: Mutex<Vec<Uuid>>,
        fail: bool,
    }

    #[async_trait]
    impl TokenRepository for SpyTokenRepo {
        async fn blacklist_token(
            &self,
            _t: String,
            _u: Uuid,
            _e: DateTime<Utc>,
        ) -> Result<(), TokenRepositoryError> {
            unimplemented!()
        }
        async fn is_token_blacklisted(&self, _t: &str) -> Result<bool, TokenRepositoryError> {
            unimplemented!()
        }
        async fn remove_blacklisted_token(&self, _t: &str) -> Result<(), TokenRepositoryError> {
            unimplemented!()
        }
        async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), TokenRepositoryError> {
            self.revoked.lock().unwrap().push(user_id);
            if self.fail {
                return Err(TokenRepositoryError::DatabaseError("redis down".into()));
            }
            Ok(())
        }
        async fn cleanup_expired_tokens(&self) -> Result<u64, TokenRepositoryError> {
            unimplemented!()
        }
    }

    // Delegating impls so a spy can be shared with the test after being handed
    // to the use case, which takes its collaborators by value.
    #[async_trait]
    impl UserRepository for Arc<SpyRepo> {
        async fn set_profile(
            &self,
            user_id: Uuid,
            full_name: String,
            _bio: Option<Option<String>>,
            _locale: Option<String>,
        ) -> Result<UserResult, UserRepositoryError> {
            self.set_full_name(user_id, full_name).await
        }

        async fn create_user(&self, d: CreateUserData) -> Result<UserResult, UserRepositoryError> {
            (**self).create_user(d).await
        }
        async fn restore_user(&self, u: Uuid) -> Result<UserResult, UserRepositoryError> {
            (**self).restore_user(u).await
        }
        async fn activate_user(&self, u: Uuid) -> Result<UserResult, UserRepositoryError> {
            (**self).activate_user(u).await
        }
        async fn set_full_name(
            &self,
            u: Uuid,
            n: String,
        ) -> Result<UserResult, UserRepositoryError> {
            (**self).set_full_name(u, n).await
        }
        async fn update_password(&self, u: Uuid, h: String) -> Result<(), UserRepositoryError> {
            (**self).update_password(u, h).await
        }
        async fn delete_user(&self, u: Uuid) -> Result<(), UserRepositoryError> {
            (**self).delete_user(u).await
        }
        async fn soft_delete_user(&self, u: Uuid) -> Result<(), UserRepositoryError> {
            (**self).soft_delete_user(u).await
        }
    }

    #[async_trait]
    impl TokenRepository for Arc<SpyTokenRepo> {
        async fn blacklist_token(
            &self,
            t: String,
            u: Uuid,
            e: DateTime<Utc>,
        ) -> Result<(), TokenRepositoryError> {
            (**self).blacklist_token(t, u, e).await
        }
        async fn is_token_blacklisted(&self, t: &str) -> Result<bool, TokenRepositoryError> {
            (**self).is_token_blacklisted(t).await
        }
        async fn remove_blacklisted_token(&self, t: &str) -> Result<(), TokenRepositoryError> {
            (**self).remove_blacklisted_token(t).await
        }
        async fn revoke_all_user_tokens(&self, u: Uuid) -> Result<(), TokenRepositoryError> {
            (**self).revoke_all_user_tokens(u).await
        }
        async fn cleanup_expired_tokens(&self) -> Result<u64, TokenRepositoryError> {
            (**self).cleanup_expired_tokens().await
        }
    }

    struct StubHasher;

    #[async_trait]
    impl PasswordHasher for StubHasher {
        async fn hash_password(&self, _p: &str) -> Result<String, HashError> {
            Ok("new-hash".to_string())
        }
        async fn verify_password(&self, _p: &str, _h: &str) -> Result<bool, HashError> {
            unimplemented!()
        }
    }

    fn service(
        token: Result<Uuid, ()>,
        repo: Arc<SpyRepo>,
        tokens: Arc<SpyTokenRepo>,
    ) -> ResetPasswordUseCase<Arc<SpyRepo>, Arc<SpyTokenRepo>> {
        ResetPasswordUseCase::new(
            repo,
            tokens,
            Arc::new(StubTokens { result: token }),
            Arc::new(StubHasher),
            Arc::new(BasicPasswordPolicy),
        )
    }

    #[tokio::test]
    async fn resets_the_password() {
        let user_id = Uuid::new_v4();
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo::default());

        service(Ok(user_id), Arc::clone(&repo), Arc::clone(&tokens))
            .execute("t", "a-long-enough-password")
            .await
            .unwrap();

        let (updated_id, hash) = repo.updated.lock().unwrap().clone().unwrap();
        assert_eq!(updated_id, user_id);
        assert_eq!(hash, "new-hash");
    }

    /// A reset is the remedy for a compromised account, so the old sessions
    /// must not survive it. Without this an attacker's refresh token keeps
    /// working after the owner believes they have locked them out.
    #[tokio::test]
    async fn every_session_is_revoked_after_a_reset() {
        let user_id = Uuid::new_v4();
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo::default());

        service(Ok(user_id), repo, Arc::clone(&tokens))
            .execute("t", "a-long-enough-password")
            .await
            .unwrap();

        assert_eq!(tokens.revoked.lock().unwrap().as_slice(), [user_id]);
    }

    /// If revocation fails the call reports an error even though the password
    /// already changed, so the outcome is not silently half-applied.
    #[tokio::test]
    async fn a_revocation_failure_is_surfaced() {
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo {
            fail: true,
            ..Default::default()
        });

        let err = service(Ok(Uuid::new_v4()), repo, tokens)
            .execute("t", "a-long-enough-password")
            .await
            .unwrap_err();

        assert!(matches!(err, ResetPasswordError::RepositoryError(_)));
    }

    /// A token of the wrong type is rejected before anything is written.
    #[tokio::test]
    async fn a_non_reset_token_changes_nothing() {
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo::default());

        let err = service(Err(()), Arc::clone(&repo), Arc::clone(&tokens))
            .execute("wrong-kind", "a-long-enough-password")
            .await
            .unwrap_err();

        assert!(matches!(err, ResetPasswordError::InvalidToken));
        assert!(repo.updated.lock().unwrap().is_none());
        assert!(tokens.revoked.lock().unwrap().is_empty());
    }

    /// The reset path must not be a way around the registration policy.
    #[tokio::test]
    async fn the_password_policy_applies_to_resets() {
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo::default());

        let err = service(Ok(Uuid::new_v4()), Arc::clone(&repo), tokens)
            .execute("t", "short")
            .await
            .unwrap_err();

        assert!(matches!(err, ResetPasswordError::InvalidPassword(_)));
        assert!(repo.updated.lock().unwrap().is_none());
    }
}
