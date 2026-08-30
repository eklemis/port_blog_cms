use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
use crate::auth::application::use_cases::create_user::CreateUserOutput;
use crate::email::application::ports::outgoing::email_sender::EmailSender;
use crate::email::application::ports::outgoing::password_reset_notifier::PasswordResetNotifier;
use crate::email::application::ports::outgoing::user_email_notifier::{
    UserEmailNotificationError, UserEmailNotifier,
};

#[derive(Clone, Debug)]
pub struct UserEmailService<T, E>
where
    T: TokenProvider + Send + Sync,
    E: EmailSender + Send + Sync,
{
    token_provider: T,
    email_sender: E,
    app_url: String,
    /// Frontend route that accepts a reset token. Kept separate from `app_url`
    /// so the two links can move independently.
    reset_url: String,
}

impl<T, E> UserEmailService<T, E>
where
    T: TokenProvider + Send + Sync,
    E: EmailSender + Send + Sync,
{
    pub fn new(
        token_provider: T,
        email_sender: E,
        app_url: String,
        reset_url: String,
    ) -> Self {
        Self {
            token_provider,
            email_sender,
            app_url,
            reset_url,
        }
    }

    fn create_password_reset_email(&self, username: &str, reset_token: &str) -> (String, String) {
        let reset_link = format!("{}/{}", self.reset_url, reset_token);

        let subject = "Reset Your Password".to_string();
        let html_body = format!(
            r#"
            <p>Hi {username},</p>
            <p>We received a request to reset your Ekstion password.</p>
            <p><a href="{reset_link}">Choose a new password</a></p>
            <p>This link expires shortly. If you did not ask for a reset you can
               ignore this email &mdash; your password will not change.</p>
            "#
        );

        (subject, html_body)
    }

    fn create_verification_email(
        &self,
        username: &str,
        verification_token: &str,
    ) -> (String, String) {
        let verification_link = format!("{}/{}", self.app_url, verification_token);

        let subject = "Verify Your Email".to_string();
        let html_body = format!(
            r#"
            <p>Hi {},</p>
            <p>Welcome to Ekstion! We're excited to have you on board.</p>
            <p>To complete your registration, click the button below:</p>
            <p>
                <a href="{}" style="display: inline-block; padding: 10px 20px; background-color: #007BFF; color: white; text-decoration: none; border-radius: 5px;">
                    Verify Your Email
                </a>
            </p>
            <p><strong>Note:</strong> This link is valid for 24 hours.</p>
            <p>Thanks,<br>The Ekstion Team</p>
            "#,
            username, verification_link
        );

        (subject, html_body)
    }
}

#[async_trait::async_trait]
impl<T, E> UserEmailNotifier for UserEmailService<T, E>
where
    T: TokenProvider + Send + Sync,
    E: EmailSender + Send + Sync,
{
    async fn send_verification_email(
        &self,
        user: CreateUserOutput,
    ) -> Result<(), UserEmailNotificationError> {
        let token = self
            .token_provider
            .generate_verification_token(user.user_id)
            .map_err(|e| UserEmailNotificationError::TokenGenerationFailed(e.to_string()))?;

        let (subject, body) = self.create_verification_email(&user.username, &token);

        self.email_sender
            .send_email(&user.email, &subject, &body)
            .await
            .map_err(UserEmailNotificationError::EmailSendingFailed)?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl<T, E> PasswordResetNotifier for UserEmailService<T, E>
where
    T: TokenProvider + Send + Sync,
    E: EmailSender + Send + Sync,
{
    async fn send_password_reset_email(
        &self,
        email: &str,
        username: &str,
        reset_token: &str,
    ) -> Result<(), UserEmailNotificationError> {
        let (subject, body) = self.create_password_reset_email(username, reset_token);

        self.email_sender
            .send_email(email, &subject, &body)
            .await
            .map_err(|e| UserEmailNotificationError::EmailSendingFailed(e.to_string()))
    }
}
