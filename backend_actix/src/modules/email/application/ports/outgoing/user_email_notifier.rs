//! Notifies a newly registered user that they must confirm their address.
//!
//! # This port is why `auth` and `email` form a cycle
//!
//! `auth` depends on this trait, which is the right direction — it needs to
//! send mail and depends on an abstraction rather than on SMTP. But the trait
//! is typed on [`CreateUserOutput`], which `auth` owns, so `email` cannot be
//! read, moved or tested without `auth` in turn.
//!
//! The fix is to give this port its own small input type carrying only the
//! fields the template needs, and have `auth` construct it. See the "Two known
//! structural issues" section of `docs/ARCHITECTURE.md`.

use crate::auth::application::use_cases::create_user::CreateUserOutput;

/// Why a notification could not be delivered.
///
/// Shared with [`PasswordResetNotifier`](super::password_reset_notifier::PasswordResetNotifier).
#[derive(Debug, thiserror::Error)]
pub enum UserEmailNotificationError {
    /// The link's token could not be minted, so no message was attempted.
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
    /// Mints a verification token and mails the confirmation link.
    ///
    /// The link points at `VERIFICATION_HANDLER_URL` with the token appended
    /// as a path segment.
    async fn send_verification_email(
        &self,
        user: CreateUserOutput,
    ) -> Result<(), UserEmailNotificationError>;
}
