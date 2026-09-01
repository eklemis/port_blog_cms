//! Password hashing port.
//!
//! Kept behind a trait so the cost parameters are a deployment decision rather
//! than a compile-time one, and so use cases can be tested without paying for
//! a real Argon2 hash on every case.

use async_trait::async_trait;

/// Why a hashing operation failed.
///
/// **None of these mean "wrong password".** A password that simply does not
/// match is `Ok(false)` from [`PasswordHasher::verify_password`]; every variant
/// here is an infrastructure fault and should surface as a 500.
#[derive(Debug, Clone, thiserror::Error)]
pub enum HashError {
    /// The hash could not be produced — bad parameters, or the allocation for
    /// the memory cost failed.
    #[error("Password hashing failed")]
    HashFailed,

    /// The stored hash could not be parsed: it is corrupt, truncated, or was
    /// written by an algorithm this implementation does not understand.
    /// Distinct from a password mismatch.
    #[error("Password verification failed")]
    VerifyFailed,

    /// The blocking task carrying the hash was cancelled or panicked. Only
    /// reachable when the implementation offloads to a thread pool.
    #[error("Background task failed")]
    TaskFailed,
}

/// Hashes and verifies passwords.
///
/// Implementations are expected to be deliberately slow — that is the point —
/// and to offload to a blocking pool when the cost is high enough to starve
/// the async runtime.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    /// Hashes a plaintext password, returning an encoded string that carries
    /// its own salt and parameters.
    ///
    /// Two calls with the same input return different strings, because the
    /// salt is fresh each time. Never compare two hashes for equality; use
    /// [`verify_password`](Self::verify_password).
    async fn hash_password(&self, password: &str) -> Result<String, HashError>;

    /// Checks a plaintext password against a stored hash.
    ///
    /// Returns `Ok(false)` for a wrong password — that is the expected
    /// negative result, not an error. An [`Err`] means the hash could not be
    /// evaluated at all.
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, HashError>;
}
