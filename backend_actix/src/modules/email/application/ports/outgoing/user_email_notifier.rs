//! Notifies a newly registered user that they must confirm their address.
//!
//! `auth` depends on this trait, which is the right direction: it needs to send
//! mail and depends on an abstraction rather than on SMTP. Nothing here depends
//! back on `auth` — the port speaks [`Recipient`], which `email` owns, and takes
//! the token already minted rather than minting it.
//!
//! It did not always. See `docs/adr/0005-break-the-auth-email-cycle.md`.

use crate::email::application::ports::outgoing::Recipient;

/// Why a notification could not be delivered.
///
/// Shared with [`PasswordResetNotifier`](super::password_reset_notifier::PasswordResetNotifier).
#[derive(Debug, thiserror::Error)]
pub enum UserEmailNotificationError {
    /// The link's token could not be minted, so no message was attempted.
    ///
    /// Produced by the caller before it reaches a notifier, not by the
    /// notifier itself — this module no longer mints tokens.
    #[error("Token generation failed: {0}")]
    TokenGenerationFailed(String),

    /// The message was composed but the transport refused it.
    ///
    /// Registration treats this as non-fatal — the account exists and
    /// verification can be resent — so callers should not roll back on it.
    #[error("Email sending failed: {0}")]
    EmailSendingFailed(String),
}

/// Sends the post-registration verification mail.
#[async_trait::async_trait]
pub trait UserEmailNotifier: Send + Sync {
    /// Mails the confirmation link.
    ///
    /// The link points at `VERIFICATION_HANDLER_URL` with
    /// `verification_token` appended as a path segment. The token is minted by
    /// the caller — `auth` owns token minting, and taking it as an argument is
    /// what keeps this module independent of `auth`.
    async fn send_verification_email(
        &self,
        recipient: &Recipient,
        verification_token: &str,
    ) -> Result<(), UserEmailNotificationError>;
}
