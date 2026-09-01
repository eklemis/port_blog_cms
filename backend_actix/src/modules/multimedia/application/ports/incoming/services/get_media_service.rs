//! Reads one media item's details.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::ports::incoming::use_cases::{
    GetMediaError, GetMediaUseCase, MediaDetail,
};
use crate::multimedia::application::ports::outgoing::db::MediaQuery;

/// Reads one media item's details.
pub struct GetMediaService<Q>
where
    Q: MediaQuery,
{
    media_query: Q,
}

impl<Q> GetMediaService<Q>
where
    Q: MediaQuery,
{
    /// Builds the service from a media reader.
    pub fn new(media_query: Q) -> Self {
        Self { media_query }
    }
}

#[async_trait]
impl<Q> GetMediaUseCase for GetMediaService<Q>
where
    Q: MediaQuery + Send + Sync,
{
    async fn execute(&self, owner: UserId, media_id: Uuid) -> Result<MediaDetail, GetMediaError> {
        // `get_attachment_info` is not owner-scoped, so ownership is enforced
        // here. A mismatch is reported as not-found rather than forbidden, to
        // match the delete endpoint and avoid confirming that an id exists.
        let attachment = self
            .media_query
            .get_attachment_info(media_id)
            .await
            .map_err(GetMediaError::from)?;

        if attachment.owner != owner {
            return Err(GetMediaError::MediaNotFound);
        }

        Ok(MediaDetail::from(attachment))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo,
    };
    use crate::multimedia::application::ports::outgoing::db::{
        MediaAttachment, MediaQueryError, StoredVariant,
    };

    struct MockQuery {
        result: Result<MediaAttachment, MediaQueryError>,
    }

    #[async_trait]
    impl MediaQuery for MockQuery {
        async fn find_public_variant(
            &self,
            _media_id: Uuid,
            _size: MediaSize,
        ) -> Result<Option<StoredVariant>, MediaQueryError> {
            Ok(None)
        }

        async fn get_state(&self, _media_id: Uuid) -> Result<MediaStateInfo, MediaQueryError> {
            unimplemented!()
        }
        async fn list_by_target(
            &self,
            _owner: UserId,
            _target: AttachmentTarget,
        ) -> Result<Vec<MediaAttachment>, MediaQueryError> {
            unimplemented!()
        }
        async fn get_attachment_info(
            &self,
            _media_id: Uuid,
        ) -> Result<MediaAttachment, MediaQueryError> {
            self.result.clone()
        }
    }

    fn attachment(owner: UserId, sizes: Vec<MediaSize>) -> MediaAttachment {
        MediaAttachment {
            media_id: Uuid::new_v4(),
            owner,
            attachment_target: AttachmentTarget::Resume,
            attachment_target_id: Uuid::new_v4(),
            status: MediaState::Ready,
            role: MediaRole::Profile,
            position: 0,
            alt_text: "alt".into(),
            caption: "cap".into(),
            original_filename: "photo.png".into(),
            variants: sizes
                .into_iter()
                .map(|size| StoredVariant {
                    size,
                    bucket_name: "blogport-cms-ready".into(),
                    object_name: "obj".into(),
                    width: 100,
                    height: 100,
                    file_size_bytes: 1234,
                    mime_type: "image/webp".into(),
                })
                .collect(),
        }
    }

    #[tokio::test]
    async fn returns_media_with_its_available_sizes() {
        let owner = UserId::from(Uuid::new_v4());
        let svc = GetMediaService::new(MockQuery {
            result: Ok(attachment(
                owner,
                vec![MediaSize::Thumbnail, MediaSize::Large],
            )),
        });

        let detail = svc.execute(owner, Uuid::new_v4()).await.unwrap();

        assert_eq!(detail.status, MediaState::Ready);
        assert_eq!(
            detail.available_sizes,
            vec![MediaSize::Thumbnail, MediaSize::Large]
        );
    }

    /// Media still processing has no variants yet; the caller polls this
    /// endpoint until `available_sizes` is populated.
    #[tokio::test]
    async fn pending_media_reports_no_available_sizes() {
        let owner = UserId::from(Uuid::new_v4());
        let mut a = attachment(owner, vec![]);
        a.status = MediaState::Pending;

        let svc = GetMediaService::new(MockQuery { result: Ok(a) });
        let detail = svc.execute(owner, Uuid::new_v4()).await.unwrap();

        assert_eq!(detail.status, MediaState::Pending);
        assert!(detail.available_sizes.is_empty());
    }

    #[tokio::test]
    async fn another_users_media_is_reported_as_not_found() {
        let owner = UserId::from(Uuid::new_v4());
        let caller = UserId::from(Uuid::new_v4());
        let svc = GetMediaService::new(MockQuery {
            result: Ok(attachment(owner, vec![])),
        });

        let err = svc.execute(caller, Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, GetMediaError::MediaNotFound));
    }

    #[tokio::test]
    async fn missing_media_is_not_found() {
        let svc = GetMediaService::new(MockQuery {
            result: Err(MediaQueryError::MediaNotFound),
        });

        let err = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, GetMediaError::MediaNotFound));
    }

    #[tokio::test]
    async fn surfaces_query_errors() {
        let svc = GetMediaService::new(MockQuery {
            result: Err(MediaQueryError::DatabaseError("db down".into())),
        });

        let err = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, GetMediaError::QueryError(m) if m == "db down"));
    }
}
