/// Separate from `UserEmailNotifier` because that trait's method takes a
/// `CreateUserOutput` — a registration-shaped payload. A reset can be requested
/// long after signup, so it needs only an address, a name to greet, and a token.
#[async_trait::async_trait]
pub trait PasswordResetNotifier: Send + Sync {
    /// Mints a reset token and mails the link.
    ///
    /// The link points at `PASSWORD_RESET_HANDLER_URL` with the token appended
    /// as a path segment. Reset tokens are shorter-lived than verification
    /// tokens by default, because the link is a live credential for the
    /// account.
    ///
    /// Callers must send this the same way whether or not the address is
    /// registered — a request that quietly succeeds for an unknown address is
    /// what stops this endpoint being an account-existence oracle.
    async fn send_password_reset_email(
        &self,
        email: &str,
        username: &str,
        reset_token: &str,
    ) -> Result<(), super::user_email_notifier::UserEmailNotificationError>;
}
