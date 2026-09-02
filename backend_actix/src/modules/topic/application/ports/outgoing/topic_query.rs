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
    /// Primary key.
    pub id: Uuid,
    /// The user this topic belongs to.
    pub owner: UserId,
    /// Display title. Unique per owner.
    pub title: String,
    /// Long-form description. Empty rather than absent when unset.
    pub description: String,
    /// When the topic was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When it was last edited.
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

/// How many things reference a topic.
///
/// What a retire-confirmation needs: "Retire «Rust»? It's on 6 posts and 2
/// projects" rather than a generic warning or an invented number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct TopicUsage {
    /// Blog posts carrying this topic, deleted ones excluded.
    #[schema(example = 6)]
    pub posts: u64,
    /// Projects carrying this topic, deleted ones excluded.
    #[schema(example = 2)]
    pub projects: u64,
}

/// Reads topics.
#[async_trait]
pub trait TopicQuery: Send + Sync {
    /// Every topic owned by `owner`, excluding soft-deleted ones.
    ///
    /// Topics are per-user, so this never returns another user's topics and
    /// two users may hold the same title.
    async fn get_topics(&self, owner: UserId) -> Result<Vec<TopicQueryResult>, TopicQueryError>;

    /// How many posts and projects carry this topic.
    ///
    /// Scoped by owner, and counts only live rows: a soft-deleted post is not
    /// a reason to keep a topic. An unused topic is `{0, 0}`, not an error.
    async fn get_topic_usage(
        &self,
        owner: UserId,
        topic_id: Uuid,
    ) -> Result<TopicUsage, TopicQueryError>;
}
