use std::sync::Arc;

use crate::auth::application::use_cases::create_user::{
    CreateUserError, CreateUserInput, CreateUserOutput, ICreateUserUseCase,
};
use crate::email::application::ports::outgoing::user_email_notifier::UserEmailNotifier;

// ============================================================================
// Registration Output with Message
// ============================================================================
#[derive(Debug)]
pub struct UserRegistrationOutput {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub username: String,
    pub full_name: String,
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

#[derive(Debug, thiserror::Error)]
pub enum UserRegistrationError {
    #[error("User creation failed: {0}")]
    CreateUserFailed(#[from] CreateUserError),

    #[error("Token generation failed: {0}")]
    TokenGenerationFailed(String),

    #[error("Email sending failed: {0}")]
    EmailSendingFailed(String),
}

// ============================================================================
// User Registration Service (Orchestration Layer)
// ============================================================================

#[derive(Clone)]
pub struct UserRegistrationOrchestrator {
    create_user_use_case: Arc<dyn ICreateUserUseCase + Send + Sync>,
    email_service: Arc<dyn UserEmailNotifier + Send + Sync>,
}

impl UserRegistrationOrchestrator {
    pub fn new(
        create_user_use_case: Arc<dyn ICreateUserUseCase + Send + Sync>,
        email_service: Arc<dyn UserEmailNotifier + Send + Sync>,
    ) -> Self {
        Self {
            create_user_use_case,
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

        // Step 2: Send verification email
        // Note: We log the error but don't fail the registration
        // The user is already created, so it's better to succeed with a warning
        if let Err(e) = self
            .email_service
            .send_verification_email(created_user.clone())
            .await
        {
            tracing::error!(
                "Failed to send verification email to user {} ({}): {}",
                created_user.user_id,
                created_user.email,
                e
            );

            // In production, you might want to:
            // - Queue this for retry
            // - Send an alert to ops team
            // - Store in dead letter queue

            // For now, we'll still return success but with a different message
            return Ok(UserRegistrationOutput {
                user_id: created_user.user_id,
                email: created_user.email,
                username: created_user.username,
                full_name: created_user.full_name,
                message: "User created successfully. We're having trouble sending the verification email. Please contact support.".to_string(),
            });
        }

        // Both user creation and email sending succeeded
        Ok(created_user.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::email::application::ports::outgoing::user_email_notifier::UserEmailNotificationError;

    use super::*;
    use async_trait::async_trait;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
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
    }

    impl MockUserEmailNotifier {
        fn new(should_fail: bool) -> Self {
            Self {
                should_fail,
                called: Arc::new(AtomicBool::new(false)),
            }
        }

        fn was_called(&self) -> bool {
            self.called.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl UserEmailNotifier for MockUserEmailNotifier {
        async fn send_verification_email(
            &self,
            _user: CreateUserOutput,
        ) -> Result<(), UserEmailNotificationError> {
            self.called.store(true, Ordering::SeqCst);

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
            Arc::new(email_notifier.clone()),
        );

        let result = service.register_user(valid_input()).await;

        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.email, "valid@example.com");
        assert!(output.message.contains("check your email"));

        assert!(
            email_notifier.was_called(),
            "Verification email should be sent"
        );
    }

    // =====================================================
    // ⚠️ SUCCESS: user created, email FAILED
    // =====================================================

    #[tokio::test]
    async fn register_user_email_failure_returns_success_with_warning() {
        let create_uc = MockCreateUserUseCase {
            result: Ok(created_user()),
        };

        let email_notifier = MockUserEmailNotifier::new(true);

        let service = UserRegistrationOrchestrator::new(
            Arc::new(create_uc),
            Arc::new(email_notifier.clone()),
        );

        let result = service.register_user(valid_input()).await;

        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(
            output
                .message
                .contains("trouble sending the verification email"),
            "Expected warning message"
        );

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
