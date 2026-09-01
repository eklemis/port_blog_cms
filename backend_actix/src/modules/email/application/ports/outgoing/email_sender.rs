//! The transport port: how a message actually leaves the process.
//!
//! Sits below the notifier ports, which decide *what* to send. SMTP in
//! production, a local Mailpit under `RUST_ENV=test`.

use async_trait::async_trait;

/// Sends one message.
#[async_trait]
pub trait EmailSender: Send + Sync {
    /// Delivers `body` to `to`.
    ///
    /// `body` is plain text; nothing here composes MIME or attachments.
    ///
    /// # Errors
    /// The error is a bare `String` rather than a typed enum, so callers can
    /// log it but cannot branch on it. Every failure mode — unreachable relay,
    /// rejected credentials, refused recipient — arrives the same way. Worth
    /// tightening if any caller ever needs to distinguish them; none does
    /// today, because every caller treats a send failure as a 500.
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
}
