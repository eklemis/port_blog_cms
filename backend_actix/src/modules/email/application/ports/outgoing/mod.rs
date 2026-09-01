//! What email needs from the outside: a transport, plus the two notification contracts auth depends on.

pub mod email_sender;
pub mod password_reset_notifier;
pub mod user_email_notifier;
pub use email_sender::EmailSender;
