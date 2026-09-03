//! Supplies `blog` with signed image URLs for a previewed draft, without
//! `blog` learning the media schema.
//!
//! `blog` declares `PreviewMediaResolver`; this implements it. The token has
//! already been checked by the time we are called — what this adds is that the
//! media must actually hang off the post that token opens, which is what keeps
//! one draft's link from resolving another post's images.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::blog::application::ports::outgoing::PreviewMediaResolver;
use crate::multimedia::application::domain::entities::{AttachmentTarget, MediaSize};
use crate::multimedia::application::ports::outgoing::cloud_storage::{MediaInfo, StorageQuery};
use crate::multimedia::application::ports::outgoing::db::MediaQuery;

/// Resolves preview images from the media tables and signs them.
pub struct PreviewMediaResolverMedia<Q, S> {
    query: Q,
    storage: S,
}

impl<Q, S> PreviewMediaResolverMedia<Q, S> {
    /// Builds it from the ports it depends on.
    pub fn new(query: Q, storage: S) -> Self {
        Self { query, storage }
    }
}

#[async_trait]
impl<Q, S> PreviewMediaResolver for PreviewMediaResolverMedia<Q, S>
where
    Q: MediaQuery + Send + Sync,
    S: StorageQuery + Send + Sync,
{
    async fn resolve(
        &self,
        post_id: Uuid,
        media_id: Uuid,
        size: &str,
    ) -> Result<Option<String>, String> {
        // An unparseable size is a 404 rather than a 500: it is a reader's URL
        // segment, not a caller's bug.
        let Ok(size) = MediaSize::from_str(size) else {
            return Ok(None);
        };

        let variant = self
            .query
            .find_variant_attached_to(media_id, size, AttachmentTarget::BlogPost, post_id)
            .await
            .map_err(|e| e.to_string())?;

        let Some(variant) = variant else {
            return Ok(None);
        };

        let info = MediaInfo::try_new(
            variant.bucket_name,
            variant.object_name,
            AttachmentTarget::BlogPost,
        )
        .map_err(|e| e.to_string())?;

        self.storage
            .get_signed_read_url(info)
            .await
            .map(Some)
            .map_err(|e| e.to_string())
    }
}

/// Boxes the resolver for the composition root.
pub fn preview_media_resolver<Q, S>(query: Q, storage: S) -> Arc<dyn PreviewMediaResolver>
where
    Q: MediaQuery + Send + Sync + 'static,
    S: StorageQuery + Send + Sync + 'static,
{
    Arc::new(PreviewMediaResolverMedia::new(query, storage))
}
