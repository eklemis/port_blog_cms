//! Re-sending the email-verification link.
//!
//! Registration mails the link exactly once and the token lasts
//! `JWT_VERIFICATION_EXPIRY` (24 hours by default). Re-registering with the
//! same address returns `USER_ALREADY_EXISTS`, so before this existed an
//! account whose owner took a day to check their mail was permanently
//! unusable — the only dead end in the product reachable by ordinary
//! behaviour.

use async_trait::async_trait;
use std::sync::Arc;

use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
use crate::auth::application::ports::outgoing::user_query::UserQuery;
use crate::email::application::ports::outgoing::user_email_notifier::UserEmailNotifier;
use crate::email::application::ports::outgoing::Recipient;

/// Why a resend request failed.
///
/// Note what is *not* here, exactly as in
/// [`RequestPasswordResetError`](super::request_password_reset::RequestPasswordResetError):
/// an unknown address, a deleted account and an already-verified one are all
/// `Ok(())`. Reporting any of them would turn this endpoint into an oracle for
/// which addresses are registered and which are confirmed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResendVerificationEmailError {
    /// The address is not well formed. Rejected before any lookup, so it
    /// reveals nothing about who is registered.
    #[error("Invalid email: {0}")]
    InvalidEmail(String),

    /// The user store could not be reached.
    #[error("Query error: {0}")]
    QueryError(String),
}

/// Mints a fresh verification token and mails the link again.
#[async_trait]
pub trait IResendVerificationEmailUseCase: Send + Sync {
    /// Sends a new verification link, if the address needs one.
    ///
    /// Succeeds whether or not the address belongs to an account, whether or
    /// not that account is already verified, and whether or not the mail could
    /// actually be sent. The caller learns nothing either way — that is the
    /// point, and it is why the route answers `202` rather than `200`.
    async fn execute(&self, email: &str) -> Result<(), ResendVerificationEmailError>;
}

/// The default implementation, generic over the user reader.
pub struct ResendVerificationEmailUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    user_query: Q,
    token_provider: Arc<dyn TokenProvider + Send + Sync>,
    notifier: Arc<dyn UserEmailNotifier + Send + Sync>,
}

impl<Q> ResendVerificationEmailUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    /// Builds the use case from its ports.
    pub fn new(
        user_query: Q,
        token_provider: Arc<dyn TokenProvider + Send + Sync>,
        notifier: Arc<dyn UserEmailNotifier + Send + Sync>,
    ) -> Self {
        Self {
            user_query,
            token_provider,
            notifier,
        }
    }
}

#[async_trait]
impl<Q> IResendVerificationEmailUseCase for ResendVerificationEmailUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    async fn execute(&self, email: &str) -> Result<(), ResendVerificationEmailError> {
        let trimmed = email.trim().to_lowercase();

        if trimmed.is_empty() {
            return Err(ResendVerificationEmailError::InvalidEmail(
                "Email cannot be empty".to_string(),
            ));
        }

        let user = self
            .user_query
            .find_by_email(&trimmed)
            .await
            .map_err(|e| ResendVerificationEmailError::QueryError(e.to_string()))?;

        let Some(user) = user else {
            tracing::info!("Verification resend requested for an address with no account");
            return Ok(());
        };

        if user.is_deleted {
            tracing::info!("Verification resend requested for a deleted account");
            return Ok(());
        }

        // Already confirmed: nothing to send, and saying so would reveal the
        // account's state. Silently succeeding also means a user who clicks an
        // old link and then hits "resend" is not told they wasted a step.
        if user.is_verified {
            tracing::info!("Verification resend requested for an already-verified account");
            return Ok(());
        }

        let token = match self.token_provider.generate_verification_token(user.id) {
            Ok(t) => t,
            Err(e) => {
                // Swallowed like the delivery failure below: surfacing it would
                // confirm the account exists.
                tracing::error!("Failed to mint a verification token: {}", e);
                return Ok(());
            }
        };

        if let Err(e) = self
            .notifier
            .send_verification_email(&Recipient::new(&user.email, &user.username), &token)
            .await
        {
            // The user gets no email and we still answer Ok. That is the cost
            // of not leaking existence; the error is logged so the failure is
            // visible to us rather than to them.
            tracing::error!("Failed to send a verification email: {}", e);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::token_provider::{TokenClaims, TokenError};
    use crate::auth::application::ports::outgoing::user_query::{UserQueryError, UserQueryResult};
    use crate::email::application::ports::outgoing::user_email_notifier::UserEmailNotificationError;
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;

    fn a_user(verified: bool, deleted: bool) -> UserQueryResult {
        UserQueryResult {
            id: Uuid::new_v4(),
            email: "jane@example.com".into(),
            username: "jane".into(),
            password_hash: "hash".into(),
            full_name: "Jane".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_verified: verified,
            is_deleted: deleted,
            bio: None,
            locale: "en".to_string(),
        }
    }

    struct StubQuery(Option<UserQueryResult>);

    #[async_trait]
    impl UserQuery for StubQuery {
        async fn find_by_email(
            &self,
            _email: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            Ok(self.0.clone())
        }
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
        async fn find_by_username(
            &self,
            _u: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
    }

    struct StubTokens;

    impl TokenProvider for StubTokens {
        fn generate_verification_token(&self, _u: Uuid) -> Result<String, TokenError> {
            Ok("fresh-token".into())
        }
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
        fn verify_verification_token(&self, _t: &str) -> Result<Uuid, TokenError> {
            unimplemented!()
        }
        fn generate_password_reset_token(&self, _u: Uuid) -> Result<String, TokenError> {
            unimplemented!()
        }
        fn verify_password_reset_token(&self, _t: &str) -> Result<Uuid, TokenError> {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct SpyNotifier {
        sent: Mutex<Vec<(String, String)>>,
    }

    #[async_trait]
    impl UserEmailNotifier for SpyNotifier {
        async fn send_verification_email(
            &self,
            recipient: &Recipient,
            token: &str,
        ) -> Result<(), UserEmailNotificationError> {
            self.sent
                .lock()
                .unwrap()
                .push((recipient.email.clone(), token.to_string()));
            Ok(())
        }
    }

    fn service(
        user: Option<UserQueryResult>,
    ) -> (ResendVerificationEmailUseCase<StubQuery>, Arc<SpyNotifier>) {
        let spy = Arc::new(SpyNotifier::default());
        let uc = ResendVerificationEmailUseCase::new(
            StubQuery(user),
            Arc::new(StubTokens),
            Arc::clone(&spy) as Arc<dyn UserEmailNotifier + Send + Sync>,
        );
        (uc, spy)
    }

    #[tokio::test]
    async fn an_unverified_account_gets_a_fresh_link() {
        let (uc, spy) = service(Some(a_user(false, false)));

        uc.execute("jane@example.com").await.unwrap();

        let sent = spy.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0], ("jane@example.com".into(), "fresh-token".into()));
    }

    /// The whole point of the endpoint: these three cases are indistinguishable
    /// from the caller's side. Any of them reporting differently would turn the
    /// route into an oracle for which addresses are registered, and which of
    /// those are already confirmed.
    #[tokio::test]
    async fn unknown_verified_and_deleted_all_succeed_silently() {
        for (label, user) in [
            ("unknown address", None),
            ("already verified", Some(a_user(true, false))),
            ("deleted account", Some(a_user(false, true))),
        ] {
            let (uc, spy) = service(user);

            let result = uc.execute("jane@example.com").await;

            assert!(result.is_ok(), "{label} must not surface an error");
            assert!(
                spy.sent.lock().unwrap().is_empty(),
                "{label} must not send mail"
            );
        }
    }

    /// A blank address is rejected before any lookup, so the 400 reveals
    /// nothing about who is registered.
    #[tokio::test]
    async fn a_blank_address_is_rejected_without_a_lookup() {
        let (uc, spy) = service(Some(a_user(false, false)));

        let err = uc.execute("   ").await.unwrap_err();

        assert!(matches!(err, ResendVerificationEmailError::InvalidEmail(_)));
        assert!(spy.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_address_is_trimmed_and_lowercased_before_lookup() {
        let (uc, spy) = service(Some(a_user(false, false)));

        uc.execute("  JANE@Example.COM  ").await.unwrap();

        assert_eq!(spy.sent.lock().unwrap().len(), 1);
    }
}
