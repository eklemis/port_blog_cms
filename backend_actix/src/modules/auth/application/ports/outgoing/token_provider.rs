//! Token minting and verification.
//!
//! Four kinds of token are issued, separated by their `token_type` claim:
//! `access`, `refresh`, `verification` and `password_reset`. They are kept
//! distinct on purpose — a link mailed to confirm an email address must not be
//! replayable to set a password.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use uuid::Uuid;

/// Why a token could not be minted or accepted.
///
/// The distinctions matter to callers: the endpoint decides both the HTTP
/// status and how much to reveal. See `docs/API_ERRORS.md` for why the same
/// condition is a 400 at one endpoint and a 401 at another.
#[derive(Debug)]
pub enum TokenError {
    /// The `exp` claim is in the past. The token was genuine.
    TokenExpired,

    /// The `nbf` claim is in the future. In practice this is clock skew
    /// between the issuing and verifying hosts, not an attack.
    TokenNotYetValid,

    /// A well-formed token of the wrong kind — the payload names the type that
    /// was expected. A refresh token offered where an access token was
    /// required lands here.
    InvalidTokenType(String),

    /// The signature did not verify: the token was tampered with, or signed
    /// with a different secret.
    InvalidSignature,

    /// Not a JWT at all — wrong segment count, or undecodable base64.
    MalformedToken,

    /// Minting failed. A server fault, never the caller's.
    EncodingError(String),
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenError::TokenExpired => write!(f, "Token has expired"),
            TokenError::TokenNotYetValid => write!(f, "Token is not yet valid"), // ADD THIS
            TokenError::InvalidTokenType(expected) => {
                write!(f, "Invalid token type, expected: {}", expected)
            }
            TokenError::InvalidSignature => write!(f, "Invalid token signature"),
            TokenError::MalformedToken => write!(f, "Malformed token"),
            TokenError::EncodingError(msg) => write!(f, "Token encoding error: {}", msg),
        }
    }
}
impl Error for TokenError {}

/// The claims carried by every token this service issues.
///
/// `is_verified` is copied into the token at mint time, so it reflects the
/// account's state when the token was issued, not now. A user who verifies
/// their email mid-session keeps a token saying otherwise until it is
/// refreshed — endpoints that must not be fooled by that should re-read the
/// user rather than trusting the claim.
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    /// The user the token identifies.
    pub sub: Uuid,
    /// Expiry, as a Unix timestamp. Past this the token is refused.
    pub exp: i64,
    /// When the token was issued, as a Unix timestamp.
    pub iat: i64,
    /// Not-before, as a Unix timestamp. A token presented earlier than this is
    /// refused — in practice that means clock skew, not an attack.
    pub nbf: i64,
    /// Which kind of token this is: `access`, `refresh`, `verification` or
    /// `password_reset`. Verification checks it, which is what keeps the four
    /// kinds from being interchangeable.
    pub token_type: String,
    /// Whether the account's email was verified **when the token was minted**.
    /// It does not track later changes — see the struct documentation.
    pub is_verified: bool,
}

/// Mints and verifies the four kinds of token.
///
/// Verification checks the signature, `exp`, `nbf` **and** `token_type`.
/// Implementations must not accept a token of the wrong kind just because it
/// is otherwise valid — that is the property keeping the four kinds separate.
///
/// This trait is synchronous: signing and verifying an HS256 token is pure
/// CPU work with no I/O to await.
pub trait TokenProvider: Send + Sync {
    /// Mints a short-lived `access` token, the credential for normal requests.
    fn generate_access_token(&self, user_id: Uuid, is_verified: bool)
        -> Result<String, TokenError>;

    /// Mints a long-lived `refresh` token, exchangeable for a new access
    /// token. Because it lives longer, this is the one worth blacklisting on
    /// logout — see [`TokenRepository`](super::token_repository::TokenRepository).
    fn generate_refresh_token(
        &self,
        user_id: Uuid,
        is_verified: bool,
    ) -> Result<String, TokenError>;

    /// Verifies any token and returns its claims.
    ///
    /// Does **not** check `token_type` — the caller must, since this method
    /// cannot know which kind was expected. Callers wanting a specific kind
    /// should prefer the paired `verify_*` methods below.
    fn verify_token(&self, token: &str) -> Result<TokenClaims, TokenError>;

    /// Exchanges a valid refresh token for a fresh access token.
    ///
    /// Rejects anything that is not of type `refresh`. Does not consult the
    /// blacklist — that check belongs to the use case, which has the
    /// repository.
    fn refresh_access_token(&self, refresh_token: &str) -> Result<String, TokenError>;

    /// Mints a token scoped to email verification, for the mailed link.
    fn generate_verification_token(&self, user_id: Uuid) -> Result<String, TokenError>;

    /// Accepts only tokens minted by
    /// [`generate_verification_token`](Self::generate_verification_token),
    /// returning the user they identify.
    fn verify_verification_token(&self, token: &str) -> Result<Uuid, TokenError>;

    /// Mints a token scoped to password reset.
    ///
    /// Deliberately a distinct `token_type` rather than reusing the
    /// verification token: otherwise a link mailed to confirm an address could
    /// be replayed to set a password, and vice versa.
    fn generate_password_reset_token(&self, user_id: Uuid) -> Result<String, TokenError>;

    /// Accepts only tokens minted by `generate_password_reset_token`.
    fn verify_password_reset_token(&self, token: &str) -> Result<Uuid, TokenError>;
}
