use crate::auth::application::use_cases::create_user::CreateUserOutput;

#[derive(Debug, thiserror::Error)]
pub enum UserEmailNotificationError {
    #[error("Token generation failed: {0}")]
    TokenGenerationFailed(String),

    #[error("Email sending failed: {0}")]
    EmailSendingFailed(String),
}

#[async_trait::async_trait]
pub trait UserEmailNotifier: Send + Sync {
    async fn send_verification_email(
        &self,
        user: CreateUserOutput,
    ) -> Result<(), UserEmailNotificationError>;
}
