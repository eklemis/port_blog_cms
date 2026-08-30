/// Separate from `UserEmailNotifier` because that trait's method takes a
/// `CreateUserOutput` — a registration-shaped payload. A reset can be requested
/// long after signup, so it needs only an address, a name to greet, and a token.
#[async_trait::async_trait]
pub trait PasswordResetNotifier: Send + Sync {
    async fn send_password_reset_email(
        &self,
        email: &str,
        username: &str,
        reset_token: &str,
    ) -> Result<(), super::user_email_notifier::UserEmailNotificationError>;
}
