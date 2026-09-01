//! The password strength rule, as a port.
//!
//! Behind a trait so the policy is a wiring decision rather than a constant
//! buried in the registration use case, and so tests can supply a permissive
//! one instead of generating compliant passwords.

/// Decides whether a password is acceptable.
pub trait PasswordPolicy: Send + Sync {
    /// Accepts or rejects a candidate password.
    ///
    /// Called on registration and on password reset, before hashing. Returns
    /// `Ok(())` for an acceptable password; the error says which rule failed
    /// so the endpoint can tell the user something actionable.
    fn validate(&self, password: &str) -> Result<(), PasswordPolicyError>;
}

/// Which rule the password failed.
#[derive(Debug)]
pub enum PasswordPolicyError {
    /// Below the minimum length.
    TooShort,
    /// Above the maximum length. A cap matters because hashing cost grows
    /// with input, so an unbounded password is a cheap way to burn CPU.
    TooLong,
    /// Long enough but fails a composition rule.
    TooWeak,
}
