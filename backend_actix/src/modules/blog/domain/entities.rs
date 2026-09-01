//! Blog domain entities.
//!
//! Free of `utoipa` annotations and any other HTTP concern, following the
//! separation established for CV in 581c071: the wire representation lives in
//! the adapter and converts to and from these types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A blog post.
///
/// Publication is modelled by `published_at` rather than a status field:
/// `None` is a draft, `Some(t)` is published at `t`. A post can therefore be
/// scheduled by setting a future timestamp without any extra state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlogPost {
    /// Primary key.
    pub id: Uuid,
    /// The owning user.
    pub user_id: Uuid,
    /// Display title.
    pub title: String,
    /// URL segment. Unique per owner.
    pub slug: String,
    /// Short summary for listings. `None` when none was written.
    pub excerpt: Option<String>,
    /// The body.
    pub content: String,
    /// `None` is a draft; a past value is published, a future one scheduled.
    pub published_at: Option<DateTime<Utc>>,
    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it was last edited.
    pub updated_at: DateTime<Utc>,
}

impl BlogPost {
    /// Whether the post is publicly visible as of `now`.
    ///
    /// A future `published_at` is treated as scheduled, not live, so the same
    /// predicate covers drafts and scheduled posts.
    pub fn is_published_at(&self, now: DateTime<Utc>) -> bool {
        matches!(self.published_at, Some(t) if t <= now)
    }
}

/// A topic attached to a post, as read back for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlogPostTopic {
    /// Primary key.
    pub id: Uuid,
    /// Display title.
    pub title: String,
    /// Long-form description.
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(published_at: Option<DateTime<Utc>>) -> BlogPost {
        let now = Utc::now();
        BlogPost {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Hello".into(),
            slug: "hello".into(),
            excerpt: None,
            content: "body".into(),
            published_at,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn a_post_without_a_publish_date_is_a_draft() {
        assert!(!post(None).is_published_at(Utc::now()));
    }

    #[test]
    fn a_post_published_in_the_past_is_live() {
        let then = Utc::now() - chrono::Duration::hours(1);
        assert!(post(Some(then)).is_published_at(Utc::now()));
    }

    /// Scheduling is the reason this is a comparison rather than a null check:
    /// a future timestamp must not leak the post early.
    #[test]
    fn a_post_scheduled_for_the_future_is_not_yet_live() {
        let later = Utc::now() + chrono::Duration::hours(1);
        assert!(!post(Some(later)).is_published_at(Utc::now()));
    }

    #[test]
    fn a_post_published_exactly_now_is_live() {
        let now = Utc::now();
        assert!(post(Some(now)).is_published_at(now));
    }
}
