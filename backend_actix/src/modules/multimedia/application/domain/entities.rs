use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MediaState {
    Pending,
    Processing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaStateInfo {
    owner: UserId,
    media_id: Uuid,
    updated_at: String,
    status: MediaState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaSize {
    Thumbnail,
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaVariant {
    size: MediaSize,
    // targeting internal route that will provide signed url
    path: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum MediaRole {
    Avatar,
    #[default]
    Profile,
    Cover,
    Screenshoot,
    Gallery,
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttachmentTarget {
    User,
    #[default]
    Resume,
    Project,
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
