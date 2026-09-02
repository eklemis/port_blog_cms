//! Supplies `auth` with a user's avatar without `auth` learning the media
//! schema.
//!
//! `auth` declares `AvatarLoader`; this implements it using the same batched
//! loader the blog and project read paths use, so an avatar is an ordinary
//! media attachment in every respect.

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::auth::application::services::AvatarLoader;
use crate::multimedia::adapter::outgoing::db::public_media_loader::load_public_media_for;
use crate::multimedia::application::domain::entities::{AttachmentTarget, PublicMedia};

/// Loads a user's avatar from the media tables.
#[derive(Clone)]
pub struct AvatarLoaderPostgres {
    db: Arc<DatabaseConnection>,
}

impl AvatarLoaderPostgres {
    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AvatarLoader for AvatarLoaderPostgres {
    async fn load(&self, user_id: Uuid) -> Result<Option<PublicMedia>, String> {
        let media = load_public_media_for(&self.db, AttachmentTarget::User, user_id)
            .await
            .map_err(|e| e.to_string())?;

        // A user may have several attachments; the avatar is the one with that
        // role, lowest position. Anything else attached to them is not a
        // profile picture and must not be served as one.
        Ok(media
            .into_iter()
            .filter(|m| m.role.eq_ignore_ascii_case("avatar"))
            .min_by_key(|m| m.position))
    }
}
