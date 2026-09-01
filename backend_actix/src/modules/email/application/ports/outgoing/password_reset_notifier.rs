//! Ports for the email module.

/// Separate from [`UserEmailNotifier`](super::user_email_notifier::UserEmailNotifier)
/// so a caller can depend on one without gaining the other. Both now speak the
/// same shape — a [`Recipient`](super::Recipient) and an already-minted token.
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
        recipient: &super::Recipient,
        reset_token: &str,
    ) -> Result<(), super::user_email_notifier::UserEmailNotificationError>;
}
