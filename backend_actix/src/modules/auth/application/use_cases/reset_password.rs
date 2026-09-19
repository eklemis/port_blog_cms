use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::auth::application::ports::incoming::password_policy::{
    PasswordPolicy, PasswordPolicyError,
};
use crate::auth::application::ports::outgoing::password_hasher::PasswordHasher;
use crate::auth::application::ports::outgoing::token_hasher::hash_token;
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

        // A reset link is a credential, and one that travels through a mailbox
        // — forwarded, screenshotted, left in a shared browser's history. Until
        // this check it could be redeemed as many times as it was presented,
        // for the whole hour it lived, including after the owner had used it
        // and believed the matter closed. Redeeming it once must spend it.
        //
        // The record is the token's own hash, never the token: the same
        // treatment logout gives a refresh token, in the same store.
        let redeemed = hash_token(token);

        match self.token_repository.is_token_blacklisted(&redeemed).await {
            Ok(true) => return Err(ResetPasswordError::InvalidToken),
            Ok(false) => {}
            // Fail closed. A reset is the remedy for a compromised account, so
            // a store that cannot say whether this link was already spent must
            // not be read as "not spent".
            Err(e) => {
                tracing::error!("Could not check whether a reset token was spent: {e}");
                return Err(ResetPasswordError::RepositoryError(format!(
                    "could not verify token state: {e}"
                )));
            }
        }

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

        // Spent last, and that order is load-bearing rather than tidy.
        // `revoke_all_user_tokens` deletes this user's blacklist entries, so a
        // token spent before that call would be un-spent by it and work again.
        // See the note on that method: it does not do what its name says.
        //
        // The entry expires with the token, because after that the token is
        // refused on its own expiry and the record is dead weight.
        let expires_at = match self.token_provider.verify_token(token) {
            Ok(claims) => DateTime::from_timestamp(claims.exp, 0).unwrap_or_else(Utc::now),
            // Unreachable: the same token verified at the top of this method.
            Err(_) => return Err(ResetPasswordError::InvalidToken),
        };

        if let Err(e) = self
            .token_repository
            .blacklist_token(redeemed, user_id, expires_at)
            .await
        {
            // The password is already changed and the sessions are already
            // gone, so failing here would report a reset that did happen as a
            // failure and invite a retry. It is logged instead — loudly,
            // because the link stays redeemable until it expires.
            tracing::error!(
                "Password reset for {} succeeded but the token could not be marked spent, \
                 so the emailed link stays usable until it expires: {}",
                user_id,
                e
            );
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
            // Read for the expiry, so the record of a spent token can expire
            // with the token rather than outliving it.
            match self.result {
                Ok(sub) => Ok(TokenClaims {
                    sub,
                    exp: (Utc::now() + chrono::Duration::hours(1)).timestamp(),
                    iat: Utc::now().timestamp(),
                    nbf: Utc::now().timestamp(),
                    token_type: "password_reset".to_string(),
                    is_verified: false,
                }),
                Err(()) => Err(TokenError::MalformedToken),
            }
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
        /// Hashes marked spent, in the order they were marked.
        spent: Mutex<Vec<String>>,
        /// Treated as already spent when the call comes in.
        already_spent: Mutex<Vec<String>>,
        /// What the store reports when asked whether a token was spent.
        lookup_fails: bool,
    }

    #[async_trait]
    impl TokenRepository for SpyTokenRepo {
        async fn blacklist_token(
            &self,
            t: String,
            _u: Uuid,
            _e: DateTime<Utc>,
        ) -> Result<(), TokenRepositoryError> {
            self.spent.lock().unwrap().push(t);
            Ok(())
        }
        async fn is_token_blacklisted(&self, t: &str) -> Result<bool, TokenRepositoryError> {
            if self.lookup_fails {
                return Err(TokenRepositoryError::DatabaseError("redis down".into()));
            }
            Ok(self.already_spent.lock().unwrap().iter().any(|h| h == t))
        }
        async fn remove_blacklisted_token(&self, _t: &str) -> Result<(), TokenRepositoryError> {
            unimplemented!()
        }
        async fn revoked_before(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<DateTime<Utc>>, TokenRepositoryError> {
            Ok(None)
        }
        async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), TokenRepositoryError> {
            self.revoked.lock().unwrap().push(user_id);
            // Mirrors the real Redis implementation, which deletes this user's
            // blacklist entries. A token spent before this call is un-spent by
            // it, which is why the order in `execute` matters.
            self.spent.lock().unwrap().clear();
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
        async fn revoked_before(
            &self,
            u: Uuid,
        ) -> Result<Option<DateTime<Utc>>, TokenRepositoryError> {
            (**self).revoked_before(u).await
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

    /// Redeeming a link spends it.
    ///
    /// The link travels by email: forwarded, screenshotted, left in a shared
    /// browser's history. Before this it could be redeemed repeatedly for its
    /// whole hour, including after the owner had used it.
    #[tokio::test]
    async fn a_redeemed_token_is_spent() {
        let user_id = Uuid::new_v4();
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo::default());

        service(Ok(user_id), Arc::clone(&repo), Arc::clone(&tokens))
            .execute("t", "a-long-enough-password")
            .await
            .expect("the first redemption succeeds");

        let spent = tokens.spent.lock().unwrap().clone();
        assert_eq!(
            spent,
            vec![hash_token("t")],
            "the token's hash is recorded, and the token itself never is"
        );
    }

    /// The second attempt with the same link changes nothing.
    #[tokio::test]
    async fn a_spent_token_is_refused_and_writes_nothing() {
        let user_id = Uuid::new_v4();
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo::default());
        tokens.already_spent.lock().unwrap().push(hash_token("t"));

        let result = service(Ok(user_id), Arc::clone(&repo), Arc::clone(&tokens))
            .execute("t", "a-different-long-password")
            .await;

        assert!(
            matches!(result, Err(ResetPasswordError::InvalidToken)),
            "a spent link must read as an invalid one: {result:?}"
        );
        assert!(
            repo.updated.lock().unwrap().is_none(),
            "a refused redemption must not change the password"
        );
        assert!(
            tokens.revoked.lock().unwrap().is_empty(),
            "nor end anyone's session"
        );
    }

    /// Spending is recorded after revocation, not before.
    ///
    /// `revoke_all_user_tokens` deletes this user's blacklist entries, so a
    /// token marked spent before that call is un-spent by it and works again.
    /// The spy clears its record on revocation for exactly this reason: if the
    /// order in `execute` is swapped, this test fails.
    #[tokio::test]
    async fn spending_survives_the_revocation_that_follows_it() {
        let user_id = Uuid::new_v4();
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo::default());

        service(Ok(user_id), repo, Arc::clone(&tokens))
            .execute("t", "a-long-enough-password")
            .await
            .unwrap();

        assert_eq!(
            tokens.spent.lock().unwrap().clone(),
            vec![hash_token("t")],
            "the record of a spent token must outlive the revocation step"
        );
    }

    /// A store that cannot answer must not be read as "not spent".
    #[tokio::test]
    async fn an_unreadable_store_refuses_the_reset() {
        let user_id = Uuid::new_v4();
        let repo = Arc::new(SpyRepo::default());
        let tokens = Arc::new(SpyTokenRepo {
            lookup_fails: true,
            ..Default::default()
        });

        let result = service(Ok(user_id), Arc::clone(&repo), tokens)
            .execute("t", "a-long-enough-password")
            .await;

        assert!(
            matches!(result, Err(ResetPasswordError::RepositoryError(_))),
            "fail closed: {result:?}"
        );
        assert!(
            repo.updated.lock().unwrap().is_none(),
            "and change nothing on the way"
        );
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
