use async_trait::async_trait;
use std::sync::Arc;

use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
use crate::auth::application::ports::outgoing::user_query::UserQuery;
use crate::email::application::ports::outgoing::password_reset_notifier::PasswordResetNotifier;

#[derive(Debug, Clone, thiserror::Error)]
pub enum RequestPasswordResetError {
    #[error("Invalid email: {0}")]
    InvalidEmail(String),

    #[error("Query error: {0}")]
    QueryError(String),
}

#[async_trait]
pub trait IRequestPasswordResetUseCase: Send + Sync {
    /// Starts a password reset.
    ///
    /// Succeeds whether or not the address belongs to an account. Reporting
    /// "no such user" would turn this endpoint into an oracle for which
    /// addresses are registered, so the caller learns nothing either way.
    async fn execute(&self, email: &str) -> Result<(), RequestPasswordResetError>;
}

pub struct RequestPasswordResetUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    user_query: Q,
    token_provider: Arc<dyn TokenProvider + Send + Sync>,
    notifier: Arc<dyn PasswordResetNotifier + Send + Sync>,
}

impl<Q> RequestPasswordResetUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    pub fn new(
        user_query: Q,
        token_provider: Arc<dyn TokenProvider + Send + Sync>,
        notifier: Arc<dyn PasswordResetNotifier + Send + Sync>,
    ) -> Self {
        Self {
            user_query,
            token_provider,
            notifier,
        }
    }
}

#[async_trait]
impl<Q> IRequestPasswordResetUseCase for RequestPasswordResetUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    async fn execute(&self, email: &str) -> Result<(), RequestPasswordResetError> {
        let trimmed = email.trim().to_lowercase();

        if trimmed.is_empty() {
            return Err(RequestPasswordResetError::InvalidEmail(
                "Email cannot be empty".to_string(),
            ));
        }

        let user = self
            .user_query
            .find_by_email(&trimmed)
            .await
            .map_err(|e| RequestPasswordResetError::QueryError(e.to_string()))?;

        let Some(user) = user else {
            tracing::info!("Password reset requested for an address with no account");
            return Ok(());
        };

        if user.is_deleted {
            tracing::info!("Password reset requested for a deleted account");
            return Ok(());
        }

        let token = match self.token_provider.generate_password_reset_token(user.id) {
            Ok(t) => t,
            Err(e) => {
                // Surfacing this would also confirm the account exists, so it
                // is logged and swallowed like the delivery failure below.
                tracing::error!("Failed to mint password reset token: {}", e);
                return Ok(());
            }
        };

        if let Err(e) = self
            .notifier
            .send_password_reset_email(&user.email, &user.username, &token)
            .await
        {
            // The user gets no email and we still answer Ok. That is the cost
            // of not leaking existence; the error is logged so the failure is
            // visible on our side rather than theirs.
            tracing::error!("Failed to send password reset email: {}", e);
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

    struct MockQuery {
        result: Result<Option<UserQueryResult>, UserQueryError>,
    }

    #[async_trait]
    impl UserQuery for MockQuery {
        async fn find_by_email(
            &self,
            _e: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            self.result.clone()
        }
        async fn find_by_username(
            &self,
            _u: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
        async fn find_by_id(&self, _i: Uuid) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
    }

    struct StubTokens;

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
            Ok("reset-token".to_string())
        }
        fn verify_password_reset_token(&self, _t: &str) -> Result<Uuid, TokenError> {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct SpyNotifier {
        sent_to: Mutex<Vec<String>>,
        fail: bool,
    }

    #[async_trait]
    impl PasswordResetNotifier for SpyNotifier {
        async fn send_password_reset_email(
            &self,
            email: &str,
            _u: &str,
            _t: &str,
        ) -> Result<(), UserEmailNotificationError> {
            self.sent_to.lock().unwrap().push(email.to_string());
            if self.fail {
                return Err(UserEmailNotificationError::EmailSendingFailed("smtp".into()));
            }
            Ok(())
        }
    }

    fn a_user(is_deleted: bool) -> UserQueryResult {
        UserQueryResult {
            id: Uuid::new_v4(),
            username: "author".into(),
            email: "user@example.com".into(),
            full_name: "The User".into(),
            password_hash: "hash".into(),
            is_verified: true,
            is_deleted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn service(
        result: Result<Option<UserQueryResult>, UserQueryError>,
        notifier: Arc<SpyNotifier>,
    ) -> RequestPasswordResetUseCase<MockQuery> {
        RequestPasswordResetUseCase::new(MockQuery { result }, Arc::new(StubTokens), notifier)
    }

    #[tokio::test]
    async fn sends_a_reset_email_to_a_known_address() {
        let spy = Arc::new(SpyNotifier::default());
        let svc = service(Ok(Some(a_user(false))), Arc::clone(&spy));

        svc.execute("user@example.com").await.unwrap();

        assert_eq!(spy.sent_to.lock().unwrap().as_slice(), ["user@example.com"]);
    }

    /// An unknown address must produce the same observable outcome as a known
    /// one, or the endpoint tells an attacker which emails are registered.
    #[tokio::test]
    async fn an_unknown_address_succeeds_without_sending() {
        let spy = Arc::new(SpyNotifier::default());
        let svc = service(Ok(None), Arc::clone(&spy));

        assert!(svc.execute("nobody@example.com").await.is_ok());
        assert!(spy.sent_to.lock().unwrap().is_empty());
    }

    /// Same for a deleted account: no mail, no distinguishable response.
    #[tokio::test]
    async fn a_deleted_account_succeeds_without_sending() {
        let spy = Arc::new(SpyNotifier::default());
        let svc = service(Ok(Some(a_user(true))), Arc::clone(&spy));

        assert!(svc.execute("user@example.com").await.is_ok());
        assert!(spy.sent_to.lock().unwrap().is_empty());
    }

    /// A delivery failure is logged, not surfaced — an error here would also
    /// confirm the address exists.
    #[tokio::test]
    async fn a_delivery_failure_still_reports_success() {
        let spy = Arc::new(SpyNotifier {
            fail: true,
            ..Default::default()
        });
        let svc = service(Ok(Some(a_user(false))), Arc::clone(&spy));

        assert!(svc.execute("user@example.com").await.is_ok());
    }

    #[tokio::test]
    async fn the_address_is_normalised_before_lookup() {
        let spy = Arc::new(SpyNotifier::default());
        let svc = service(Ok(Some(a_user(false))), Arc::clone(&spy));

        assert!(svc.execute("  USER@Example.com  ").await.is_ok());
    }

    #[tokio::test]
    async fn an_empty_address_is_rejected() {
        let spy = Arc::new(SpyNotifier::default());
        let svc = service(Ok(None), spy);

        let err = svc.execute("   ").await.unwrap_err();
        assert!(matches!(err, RequestPasswordResetError::InvalidEmail(_)));
    }
}
