//! Who a notification is addressed to.

/// The fields every notification template needs about its recipient.
///
/// Owned by `email` rather than reusing a caller's DTO. That is deliberate:
/// this module used to type its notifier port on `auth`'s `CreateUserOutput`,
/// which meant `email` could not be read, moved or tested without `auth` even
/// though it is the more generic of the two. See
/// `docs/adr/0005-break-the-auth-email-cycle.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    /// Address the message is sent to.
    pub email: String,
    /// Name used to greet them in the body.
    pub username: String,
}

impl Recipient {
    /// Builds a recipient from an address and a display name.
    pub fn new(email: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            username: username.into(),
        }
    }
}
