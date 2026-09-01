use std::sync::Arc;
use std::time::Duration;

use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
use crate::auth::application::use_cases::create_user::{
    CreateUserError, CreateUserInput, CreateUserOutput, ICreateUserUseCase,
};
use crate::email::application::ports::outgoing::user_email_notifier::UserEmailNotifier;
use crate::email::application::ports::outgoing::Recipient;

// ============================================================================
// Registration Output with Message
// ============================================================================
/// See the module documentation.
#[derive(Debug)]
pub struct UserRegistrationOutput {
    /// Primary key of the newly created user.
    pub user_id: uuid::Uuid,
    /// Email address.
    pub email: String,
    /// Public handle.
    pub username: String,
    /// Display name.
    pub full_name: String,
    /// Human-readable text for the caller.
    pub message: String,
}

impl From<CreateUserOutput> for UserRegistrationOutput {
    fn from(output: CreateUserOutput) -> Self {
        Self {
            user_id: output.user_id,
            email: output.email,
            username: output.username,
            full_name: output.full_name,
            message: "User created successfully. Please check your email to verify your account."
                .to_string(),
        }
    }
}

// ============================================================================
// Registration Errors
// ============================================================================

/// See the module documentation.
#[derive(Debug, thiserror::Error)]
pub enum UserRegistrationError {
    /// The account could not be created; nothing was sent.
    #[error("User creation failed: {0}")]
    CreateUserFailed(#[from] CreateUserError),

    /// The account exists but the verification token could not be minted, so no
    /// mail was attempted. Surfaced rather than swallowed — see
    /// `docs/adr/0005-break-the-auth-email-cycle.md`.
    #[error("Token generation failed: {0}")]
    TokenGenerationFailed(String),

    /// Retained for callers that treat delivery as fatal. The registration path
    /// itself sends in a detached task and does not produce this.
    #[error("Email sending failed: {0}")]
    EmailSendingFailed(String),
}

// ============================================================================
// User Registration Service (Orchestration Layer)
// ============================================================================

/// See the module documentation.
#[derive(Clone)]
pub struct UserRegistrationOrchestrator {
    create_user_use_case: Arc<dyn ICreateUserUseCase + Send + Sync>,
    /// Mints the verification token. Held here rather than inside the notifier
    /// because minting is `auth`'s job — see
    /// `docs/adr/0005-break-the-auth-email-cycle.md`.
    token_provider: Arc<dyn TokenProvider + Send + Sync>,
    email_service: Arc<dyn UserEmailNotifier + Send + Sync>,
}

impl UserRegistrationOrchestrator {
    /// Builds it from the ports it depends on.
    pub fn new(
        create_user_use_case: Arc<dyn ICreateUserUseCase + Send + Sync>,
        token_provider: Arc<dyn TokenProvider + Send + Sync>,
        email_service: Arc<dyn UserEmailNotifier + Send + Sync>,
    ) -> Self {
        Self {
            create_user_use_case,
            token_provider,
            email_service,
        }
    }

    /// Orchestrates complete user registration:
    /// 1. Creates user account
    /// 2. Sends verification email
    pub async fn register_user(
        &self,
        input: CreateUserInput,
    ) -> Result<UserRegistrationOutput, UserRegistrationError> {
        // Step 1: Create user account
        let created_user = self.create_user_use_case.execute(input).await?;

        // Step 2: mint the verification token here, before spawning. Minting is
        // synchronous and cheap, and doing it on this task means a failure is
        // visible to the caller rather than disappearing into a detached one.
        let token = self
            .token_provider
            .generate_verification_token(created_user.user_id)
            .map_err(|e| UserRegistrationError::TokenGenerationFailed(e.to_string()))?;

        // Step 3: Spawn email sending as a background task (fire-and-forget)
        let email_service = self.email_service.clone();
        let recipient = Recipient::new(&created_user.email, &created_user.username);
        // Captured for the log lines below: the spawned task must not borrow
        // `created_user`, which is returned to the caller.
        let user_id = created_user.user_id;

        tokio::spawn(async move {
            let max_retries = 3;
            for attempt in 1..=max_retries {
                match email_service
                    .send_verification_email(&recipient, &token)
                    .await
                {
                    Ok(_) => return,
                    Err(e) if attempt < max_retries => {
                        tracing::warn!(
                            "Email attempt {}/{} failed for user {}: {}. Retrying...",
                            attempt,
                            max_retries,
                            user_id,
                            e
                        );
                        tokio::time::sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                    }
                    Err(e) => {
                        tracing::error!(
                            "All {} email attempts failed for user {}: {}",
                            max_retries,
                            user_id,
                            e
                        );
                    }
                }
            }
        });

        // Return immediately - don't wait for email
        Ok(created_user.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::application::ports::outgoing::token_provider::{TokenClaims, TokenError};
    use crate::email::application::ports::outgoing::user_email_notifier::UserEmailNotificationError;

    use super::*;
    use async_trait::async_trait;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tokio::sync::Notify;
    use uuid::Uuid;

    // =====================================================
    // Mock CreateUserUseCase
    // =====================================================

    #[derive(Clone)]
    struct MockCreateUserUseCase {
        result: Result<CreateUserOutput, CreateUserError>,
    }

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserUseCase {
        async fn execute(
            &self,
            _input: CreateUserInput,
        ) -> Result<CreateUserOutput, CreateUserError> {
            self.result.clone()
        }
    }

    // =====================================================
    // Mock UserEmailNotifier
    // =====================================================

    #[derive(Clone)]
    struct MockUserEmailNotifier {
        should_fail: bool,
        called: Arc<AtomicBool>,
        notify: Arc<Notify>,
    }

    impl MockUserEmailNotifier {
        fn new(should_fail: bool) -> Self {
            Self {
                called: Arc::new(AtomicBool::new(false)),
                should_fail,
                notify: Arc::new(Notify::new()),
            }
        }

        fn was_called(&self) -> bool {
            self.called.load(Ordering::SeqCst)
        }
        async fn wait_until_called(&self) {
            self.notify.notified().await;
        }
    }

    /// Mints a fixed verification token, or fails on demand.
    ///
    /// The orchestrator mints the token now — see
    /// `docs/adr/0005-break-the-auth-email-cycle.md` — so these tests need one.
    #[derive(Clone)]
    struct StubTokenProvider {
        verification: Option<&'static str>,
    }

    impl StubTokenProvider {
        fn ok() -> Self {
            Self {
                verification: Some("verify-tok"),
            }
        }
        fn failing() -> Self {
            Self { verification: None }
        }
    }

    impl TokenProvider for StubTokenProvider {
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
            self.verification
                .map(|t| t.to_string())
                .ok_or(TokenError::MalformedToken)
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

    #[async_trait]
    impl UserEmailNotifier for MockUserEmailNotifier {
        async fn send_verification_email(
            &self,
            _recipient: &Recipient,
            _verification_token: &str,
        ) -> Result<(), UserEmailNotificationError> {
            self.called.store(true, Ordering::SeqCst);
            self.notify.notify_one();

            if self.should_fail {
                Err(UserEmailNotificationError::EmailSendingFailed(
                    "SMTP down".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }

    // =====================================================
    // Helpers
    // =====================================================

    fn valid_input() -> CreateUserInput {
        CreateUserInput {
            username: "validuser".to_string(),
            email: "valid@example.com".to_string(),
            password: "VerySecurePassword123!".to_string(),
            full_name: "Valid User".to_string(),
        }
    }

    fn created_user() -> CreateUserOutput {
        CreateUserOutput {
            user_id: Uuid::new_v4(),
            email: "valid@example.com".to_string(),
            username: "validuser".to_string(),
            full_name: "Valid User".to_string(),
        }
    }

    // =====================================================
    // ✅ SUCCESS: user created + email sent
    // =====================================================

    #[tokio::test]
    async fn register_user_success() {
        let create_uc = MockCreateUserUseCase {
            result: Ok(created_user()),
        };

        let email_notifier = MockUserEmailNotifier::new(false);

        let service = UserRegistrationOrchestrator::new(
            Arc::new(create_uc),
            Arc::new(StubTokenProvider::ok()),
            Arc::new(email_notifier.clone()),
        );

        let result = service.register_user(valid_input()).await;

        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.email, "valid@example.com");
        assert!(output.message.contains("check your email"));

        // Wait for the email task (with timeout)
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            email_notifier.wait_until_called(),
        )
        .await
        .expect("Email should have been sent within 1 second");

        assert!(email_notifier.was_called());
    }

    // =====================================================
    // ⚠️ SUCCESS: user created, email FAILED
    // =====================================================

    /// The verification token is minted here, not in the notifier, so a minting
    /// failure has to surface from `register_user` rather than vanishing into
    /// the detached email task. This test moved out of `email_service` with the
    /// dependency — see `docs/adr/0005-break-the-auth-email-cycle.md`.
    #[tokio::test]
    async fn a_token_minting_failure_fails_registration_and_sends_nothing() {
        let create_uc = MockCreateUserUseCase {
            result: Ok(created_user()),
        };
        let email_notifier = MockUserEmailNotifier::new(false);

        let service = UserRegistrationOrchestrator::new(
            Arc::new(create_uc),
            Arc::new(StubTokenProvider::failing()),
            Arc::new(email_notifier.clone()),
        );

        let err = service.register_user(valid_input()).await.unwrap_err();

        assert!(matches!(
            err,
            UserRegistrationError::TokenGenerationFailed(_)
        ));
        assert!(
            !email_notifier.was_called(),
            "no mail should be attempted when the token could not be minted"
        );
    }

    #[tokio::test]
    async fn register_user_succeeds_even_when_email_fails() {
        let create_uc = MockCreateUserUseCase {
            result: Ok(created_user()),
        };

        let email_notifier = MockUserEmailNotifier::new(true); // will fail

        let service = UserRegistrationOrchestrator::new(
            Arc::new(create_uc),
            Arc::new(StubTokenProvider::ok()),
            Arc::new(email_notifier.clone()),
        );

        let result = service.register_user(valid_input()).await;

        // Registration still succeeds
        assert!(result.is_ok());

        let output = result.unwrap();
        // Standard success message (no warning - we don't know email failed yet)
        assert!(output.message.contains("check your email"));

        // Give spawned task time to run
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Email was still attempted
        assert!(
            email_notifier.was_called(),
            "Email notifier should still be called (and fail)"
        );
    }

    // =====================================================
    // ❌ FAILURE: user creation fails
    // =====================================================

    #[tokio::test]
    async fn register_user_create_user_fails() {
        let create_uc = MockCreateUserUseCase {
            result: Err(CreateUserError::UserAlreadyExists),
        };

        let email_notifier = MockUserEmailNotifier::new(false);

        let service = UserRegistrationOrchestrator::new(
            Arc::new(create_uc),
            Arc::new(StubTokenProvider::ok()),
            Arc::new(email_notifier.clone()),
        );

        let result = service.register_user(valid_input()).await;

        assert!(result.is_err());

        match result.unwrap_err() {
            UserRegistrationError::CreateUserFailed(CreateUserError::UserAlreadyExists) => {}
            other => panic!("Unexpected error: {:?}", other),
        }

        assert!(
            !email_notifier.was_called(),
            "Email should NOT be attempted if user creation fails"
        );
    }
}
