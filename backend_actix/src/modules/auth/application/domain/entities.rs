use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

impl UserId {
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_verified: bool, // ✅ Added for email verification
    pub is_deleted: bool,  // ✅ Added for soft delete
}

#[derive(Debug, Clone)]
pub struct BlacklistedToken {
    pub id: Uuid,
    pub token_hash: String,
    pub user_id: Uuid,
    pub blacklisted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl BlacklistedToken {
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
