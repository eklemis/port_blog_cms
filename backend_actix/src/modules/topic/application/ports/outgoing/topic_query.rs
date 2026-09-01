//! Read-side port for topics.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// A topic as returned to readers.
///
/// Carries every persisted field except `is_deleted` — soft-deleted topics are
/// filtered out by the query, so the flag would always be `false` here.
#[derive(Debug, Clone)]
pub struct TopicQueryResult {
    pub id: Uuid,
    pub owner: UserId,
    pub title: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Why a topic read failed.
///
/// A user with no topics is an empty `Vec`, not an error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TopicQueryError {
    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Reads topics.
#[async_trait]
pub trait TopicQuery: Send + Sync {
    /// Every topic owned by `owner`, excluding soft-deleted ones.
    ///
    /// Topics are per-user, so this never returns another user's topics and
    /// two users may hold the same title.
    async fn get_topics(&self, owner: UserId) -> Result<Vec<TopicQueryResult>, TopicQueryError>;
}
