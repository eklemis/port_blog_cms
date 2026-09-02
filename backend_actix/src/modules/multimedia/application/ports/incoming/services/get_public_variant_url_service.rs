//! Checks public visibility, then signs.

use async_trait::async_trait;
use uuid::Uuid;

use crate::multimedia::application::domain::entities::MediaSize;
use crate::multimedia::application::ports::incoming::use_cases::{
    GetPublicVariantUrlError, GetPublicVariantUrlUseCase,
};
use crate::multimedia::application::ports::outgoing::cloud_storage::{MediaInfo, StorageQuery};
use crate::multimedia::application::ports::outgoing::db::MediaQuery;

/// Signs read URLs for variants a reader is allowed to see.
#[derive(Clone)]
pub struct GetPublicVariantUrlService<Q, S> {
    query: Q,
    storage: S,
}

impl<Q, S> GetPublicVariantUrlService<Q, S> {
    /// Builds it from the ports it depends on.
    pub fn new(query: Q, storage: S) -> Self {
        Self { query, storage }
    }
}

#[async_trait]
impl<Q, S> GetPublicVariantUrlUseCase for GetPublicVariantUrlService<Q, S>
where
    Q: MediaQuery + Send + Sync,
    S: StorageQuery + Send + Sync,
{
    async fn execute(
        &self,
        media_id: Uuid,
        size: MediaSize,
    ) -> Result<String, GetPublicVariantUrlError> {
        // The visibility check is the whole point: it is what makes an
        // unpublished post's imagery stop being served.
        let variant = self
            .query
            .find_public_variant(media_id, size)
            .await
            .map_err(|e| GetPublicVariantUrlError::QueryError(e.to_string()))?
            .ok_or(GetPublicVariantUrlError::NotFound)?;

        let info = MediaInfo::try_new(
            variant.bucket_name,
            variant.object_name,
            // The target is not used to sign; the coordinates are.
            crate::multimedia::application::domain::entities::AttachmentTarget::BlogPost,
        )
        .map_err(|e| GetPublicVariantUrlError::StorageError(e.to_string()))?;

        self.storage
            .get_signed_read_url(info)
            .await
            .map_err(|e| GetPublicVariantUrlError::StorageError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multimedia::application::domain::entities::{AttachmentTarget, MediaStateInfo};
    use crate::multimedia::application::ports::outgoing::cloud_storage::{
        ManifestInfo, SignUrlError, StorageQueryError,
    };
    use crate::multimedia::application::ports::outgoing::db::{
        MediaAttachment, MediaQueryError, StoredVariant,
    };
    use std::sync::Mutex;

    /// Returns a variant only when `visible` is set — standing in for the
    /// published-post join the real adapter performs.
    struct StubQuery {
        visible: bool,
        asked: Mutex<Vec<(uuid::Uuid, MediaSize)>>,
    }

    impl StubQuery {
        fn new(visible: bool) -> Self {
            Self {
                visible,
                asked: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl crate::multimedia::application::ports::outgoing::db::MediaQuery for StubQuery {
        async fn get_states(
            &self,
            _owner: crate::auth::application::domain::entities::UserId,
            _media_ids: &[uuid::Uuid],
        ) -> Result<Vec<MediaStateInfo>, MediaQueryError> {
            Ok(Vec::new())
        }

        async fn find_media_usage(
            &self,
            _owner: crate::auth::application::domain::entities::UserId,
            _media_id: uuid::Uuid,
        ) -> Result<
            Vec<crate::multimedia::application::ports::outgoing::db::MediaUsageRow>,
            MediaQueryError,
        > {
            Ok(Vec::new())
        }

        async fn find_public_variant(
            &self,
            media_id: uuid::Uuid,
            size: MediaSize,
        ) -> Result<Option<StoredVariant>, MediaQueryError> {
            self.asked.lock().unwrap().push((media_id, size.clone()));
            Ok(self.visible.then(|| StoredVariant {
                size,
                bucket_name: "ready-bucket".into(),
                object_name: "m/large.webp".into(),
                width: 1,
                height: 1,
                file_size_bytes: 1,
                mime_type: "image/webp".into(),
            }))
        }
        async fn get_state(&self, _id: uuid::Uuid) -> Result<MediaStateInfo, MediaQueryError> {
            unimplemented!()
        }
        async fn list_by_target(
            &self,
            _owner: crate::auth::application::domain::entities::UserId,
            _target: AttachmentTarget,
        ) -> Result<Vec<MediaAttachment>, MediaQueryError> {
            unimplemented!()
        }
        async fn get_attachment_info(
            &self,
            _id: uuid::Uuid,
        ) -> Result<MediaAttachment, MediaQueryError> {
            unimplemented!()
        }
    }

    struct StubStorage {
        signed: Mutex<Vec<(String, String)>>,
    }

    #[async_trait]
    impl StorageQuery for StubStorage {
        async fn get_signed_read_url(&self, m: MediaInfo) -> Result<String, SignUrlError> {
            self.signed
                .lock()
                .unwrap()
                .push((m.bucket_name().into(), m.object_name().into()));
            Ok("https://signed.example/obj?sig=abc".into())
        }
        async fn get_signed_upload_url(&self, _m: MediaInfo) -> Result<String, SignUrlError> {
            unimplemented!()
        }
        async fn get_latest_manifest(&self, _id: &str) -> Result<ManifestInfo, StorageQueryError> {
            unimplemented!()
        }
    }

    fn storage() -> StubStorage {
        StubStorage {
            signed: Mutex::new(Vec::new()),
        }
    }

    #[tokio::test]
    async fn a_visible_variant_is_signed_from_its_stored_coordinates() {
        let svc = GetPublicVariantUrlService::new(StubQuery::new(true), storage());
        let id = uuid::Uuid::new_v4();

        let url = svc.execute(id, MediaSize::Large).await.unwrap();

        assert_eq!(url, "https://signed.example/obj?sig=abc");
        assert_eq!(
            svc.storage.signed.lock().unwrap()[0],
            ("ready-bucket".to_string(), "m/large.webp".to_string()),
            "must sign the coordinates the query returned, not guess them"
        );
    }

    /// The property a world-readable bucket cannot provide: once the post is
    /// unpublished the query stops returning the variant, and this 404s.
    #[tokio::test]
    async fn a_variant_that_is_not_publicly_visible_is_not_found() {
        let svc = GetPublicVariantUrlService::new(StubQuery::new(false), storage());

        let err = svc
            .execute(uuid::Uuid::new_v4(), MediaSize::Large)
            .await
            .unwrap_err();

        assert!(matches!(err, GetPublicVariantUrlError::NotFound));
        assert!(
            svc.storage.signed.lock().unwrap().is_empty(),
            "nothing may be signed for media the reader cannot see"
        );
    }

    #[tokio::test]
    async fn the_requested_size_reaches_the_query_unchanged() {
        let svc = GetPublicVariantUrlService::new(StubQuery::new(true), storage());
        let id = uuid::Uuid::new_v4();

        svc.execute(id, MediaSize::Thumbnail).await.unwrap();

        assert_eq!(
            svc.query.asked.lock().unwrap()[0],
            (id, MediaSize::Thumbnail)
        );
    }
}
