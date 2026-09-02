use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user's identity.
///
/// A newtype rather than a bare `Uuid` so a user id cannot be passed where a
/// post or project id is expected. Every other module imports this — it is
/// auth's published surface, and a change to it is a change to all seven.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

impl UserId {
    /// The underlying UUID, for the boundaries that need one.
    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for UserId {
    fn from(id: Uuid) -> Self {
        UserId(id)
    }
}

impl From<UserId> for Uuid {
    fn from(id: UserId) -> Self {
        id.0
    }
}

/// A user account, as the domain sees it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct User {
    /// Primary key.
    pub id: Uuid,
    /// Public handle. Appears in public URLs.
    pub username: String,
    /// Login address.
    pub email: String,
    /// Argon2 hash. Never a plaintext password.
    pub password_hash: String,
    /// Free-form public introduction. `None` when never set.
    pub bio: Option<String>,
    /// When the account was created.
    pub created_at: DateTime<Utc>,
    /// When the row was last written.
    pub updated_at: DateTime<Utc>,
    /// Whether the email address has been confirmed.
    pub is_verified: bool,
    /// Whether the account is soft-deleted. Queries do not filter on it, so
    /// callers must check.
    pub is_deleted: bool,
}

/// A refresh token that must no longer be accepted.
///
/// Stored as a hash, never in the clear — the table would otherwise be a pile
/// of working credentials.
#[derive(Debug, Clone)]
pub struct BlacklistedToken {
    /// Primary key.
    pub id: Uuid,
    /// SHA-256 of the token.
    pub token_hash: String,
    /// Whose token it was.
    pub user_id: Uuid,
    /// When it was revoked.
    pub blacklisted_at: DateTime<Utc>,
    /// The token's own expiry. Past this the entry is dead weight, which is what
    /// lets the store expire it.
    pub expires_at: DateTime<Utc>,
}

impl BlacklistedToken {
    /// Records a revocation, stamped now.
    pub fn new(token_hash: String, user_id: Uuid, expires_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            token_hash,
            user_id,
            blacklisted_at: Utc::now(),
            expires_at,
        }
    }

    /// Check if this blacklisted token has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_id_round_trips_through_uuid() {
        let raw = Uuid::new_v4();
        let id = UserId::from(raw);
        assert_eq!(id.value(), raw);
        assert_eq!(Uuid::from(id), raw);
    }

    #[test]
    fn user_ids_compare_by_value() {
        let raw = Uuid::new_v4();
        assert_eq!(UserId::from(raw), UserId::from(raw));
        assert_ne!(UserId::from(raw), UserId::from(Uuid::new_v4()));
    }

    /// UserId serialises as a bare UUID string, which is why response DTOs
    /// describe it with `value_type = String` rather than pulling it into the
    /// OpenAPI schema.
    #[test]
    fn user_id_serialises_as_a_bare_uuid() {
        let raw = Uuid::new_v4();
        let json = serde_json::to_string(&UserId::from(raw)).unwrap();
        assert_eq!(json, format!("\"{raw}\""));

        let back: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), raw);
    }

    #[test]
    fn a_blacklisted_token_records_what_it_was_given() {
        let user_id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::hours(1);
        let t = BlacklistedToken::new("hash".to_string(), user_id, expires_at);

        assert_eq!(t.token_hash, "hash");
        assert_eq!(t.user_id, user_id);
        assert_eq!(t.expires_at, expires_at);
        assert!(!t.id.is_nil(), "each entry gets its own id");
    }

    /// The blacklist is only meaningful until a token would have expired
    /// anyway, so this predicate decides when an entry can be dropped.
    #[test]
    fn expiry_is_decided_against_the_current_time() {
        let past = BlacklistedToken::new(
            "h".into(),
            Uuid::new_v4(),
            Utc::now() - chrono::Duration::seconds(1),
        );
        assert!(past.is_expired());

        let future = BlacklistedToken::new(
            "h".into(),
            Uuid::new_v4(),
            Utc::now() + chrono::Duration::hours(1),
        );
        assert!(!future.is_expired());
    }
}
