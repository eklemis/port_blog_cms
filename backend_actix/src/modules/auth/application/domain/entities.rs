use chrono::{DateTime, Utc};
use uuid::Uuid;

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
