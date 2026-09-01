use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// Where an upload is in its lifecycle.
///
/// A row is created before the bytes arrive, so a media item existing does
/// not mean the file does.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaState {
    /// Registered, but the client has not uploaded the bytes yet.
    Pending,
    /// Bytes arrived; variants are being generated. Retry shortly.
    Processing,
    /// Variants exist and can be served.
    Ready,
    /// Variant generation failed. Terminal — retrying will not help.
    Failed,
}

/// Just enough to answer "is this ready yet?" without loading the item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaStateInfo {
    /// Who uploaded it.
    pub owner: UserId,
    /// Which item.
    pub media_id: Uuid,
    /// When the state last changed.
    pub updated_at: String,
    /// The current state.
    pub status: MediaState,
}

/// A generated variant size. The processor writes one object per size.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaSize {
    /// Smallest — list rows and avatars.
    Thumbnail,
    /// Cards and inline previews.
    Small,
    /// The usual display size.
    Medium,
    /// Full-width display.
    Large,
}
impl fmt::Display for MediaSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MediaSize::Thumbnail => "thumbnail",
            MediaSize::Small => "small",
            MediaSize::Medium => "medium",
            MediaSize::Large => "large",
        };
        write!(f, "{}", s)
    }
}

/// Parsing is the inverse of [`Display`](std::fmt::Display), so the wire form
/// and the URL segment are the same string. Lives here rather than in a route
/// so every caller agrees on which sizes exist.
impl std::str::FromStr for MediaSize {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "thumbnail" => Ok(MediaSize::Thumbnail),
            "small" => Ok(MediaSize::Small),
            "medium" => Ok(MediaSize::Medium),
            "large" => Ok(MediaSize::Large),
            _ => Err(()),
        }
    }
}

/// One generated size and where its object lives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaVariant {
    /// Which size this is.
    pub size: MediaSize,
    // targeting internal route that will provide signed url
    /// Object key within the ready bucket.
    pub path: String,
}

/// What a media item is for on the thing it is attached to.
///
/// The role decides how a client renders it, and lets one target carry
/// several images without ambiguity.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum MediaRole {
    /// Small round portrait.
    Avatar,
    /// Larger portrait on a profile or CV.
    #[default]
    Profile,
    /// Wide banner at the top of a page.
    Cover,
    /// A project screenshot. **Spelling is load-bearing** — it is persisted, so
    /// correcting it needs a migration.
    Screenshoot,
    /// One of several images shown together.
    Gallery,
    /// Embedded in body content.
    Inline,
}
impl fmt::Display for MediaRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MediaRole::Avatar => "avatar",
            MediaRole::Profile => "profile",
            MediaRole::Cover => "cover",
            MediaRole::Screenshoot => "screenshoot",
            MediaRole::Gallery => "gallery",
            MediaRole::Inline => "inline",
        };
        write!(f, "{s}")
    }
}

/// What kind of thing a media item is attached to.
///
/// Persisted, so adding a variant needs a migration and removing one needs a
/// backfill.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub enum AttachmentTarget {
    /// A user account.
    User,
    /// A CV.
    #[default]
    Resume,
    /// A project.
    Project,
    /// A blog post.
    BlogPost,
}

impl fmt::Display for AttachmentTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AttachmentTarget::User => "user",
            AttachmentTarget::Resume => "resume",
            AttachmentTarget::Project => "project",
            AttachmentTarget::BlogPost => "blog_post",
        };
        write!(f, "{s}")
    }
}
