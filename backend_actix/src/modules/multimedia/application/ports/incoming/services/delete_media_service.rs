//! Deletes media rows.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::ports::incoming::use_cases::{
    DeleteMediaError, DeleteMediaUseCase,
};
use crate::multimedia::application::ports::outgoing::db::MediaRepository;

/// Deletes media rows.
pub struct DeleteMediaService<R>
where
    R: MediaRepository,
{
    media_repository: R,
}

impl<R> DeleteMediaService<R>
where
    R: MediaRepository,
{
    /// Builds the service from a media repository.
    pub fn new(media_repository: R) -> Self {
        Self { media_repository }
    }
}

#[async_trait]
impl<R> DeleteMediaUseCase for DeleteMediaService<R>
where
    R: MediaRepository + Send + Sync,
{
    async fn execute(&self, owner: UserId, media_id: Uuid) -> Result<(), DeleteMediaError> {
        self.media_repository
            .soft_delete(owner, media_id)
            .await
            .map_err(DeleteMediaError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multimedia::application::domain::entities::{MediaStateInfo, MediaVariant};
    use crate::multimedia::application::ports::outgoing::db::PatchAttachmentData;
    use crate::multimedia::application::ports::outgoing::db::{
        MediaRepositoryError, MediaVariantRecord, RecordMediaError, RecordMediaTx, RecordedMedia,
        UpdateMediaStateData,
    };

    struct MockRepo {
        result: Result<(), MediaRepositoryError>,
    }

    #[async_trait]
    impl MediaRepository for MockRepo {
        async fn patch_attachment(
            &self,
            _owner: UserId,
            _media_id: Uuid,
            _data: PatchAttachmentData,
        ) -> Result<(), MediaRepositoryError> {
            unimplemented!()
        }

        async fn restore(&self, _o: UserId, _m: Uuid) -> Result<(), MediaRepositoryError> {
            unimplemented!()
        }

        async fn hard_delete(&self, _o: UserId, _m: Uuid) -> Result<(), MediaRepositoryError> {
            unimplemented!()
        }

        async fn record_media_tx(
            &self,
            _tx: RecordMediaTx,
        ) -> Result<RecordedMedia, RecordMediaError> {
            unimplemented!()
        }
        async fn set_media_state(
            &self,
            _data: UpdateMediaStateData,
        ) -> Result<MediaStateInfo, MediaRepositoryError> {
            unimplemented!()
        }
        async fn record_single_variant(
            &self,
            _data: MediaVariantRecord,
        ) -> Result<MediaVariant, MediaRepositoryError> {
            unimplemented!()
        }
        async fn record_variants(
            &self,
            _data: Vec<MediaVariantRecord>,
        ) -> Result<Vec<MediaVariant>, MediaRepositoryError> {
            unimplemented!()
        }
        async fn soft_delete(
            &self,
            _owner: UserId,
            _media_id: Uuid,
        ) -> Result<(), MediaRepositoryError> {
            self.result.clone()
        }
    }

    fn service(result: Result<(), MediaRepositoryError>) -> DeleteMediaService<MockRepo> {
        DeleteMediaService::new(MockRepo { result })
    }

    #[tokio::test]
    async fn deletes_media() {
        let svc = service(Ok(()));
        assert!(svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .is_ok());
    }

    /// The repository reports absent and not-yours identically, and the use
    /// case keeps them merged so the endpoint cannot be used to probe for ids.
    #[tokio::test]
    async fn missing_or_foreign_media_is_not_found() {
        let svc = service(Err(MediaRepositoryError::NotFound));

        let err = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, DeleteMediaError::MediaNotFound));
    }

    #[tokio::test]
    async fn surfaces_database_errors() {
        let svc = service(Err(MediaRepositoryError::DatabaseError("db down".into())));

        let err = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, DeleteMediaError::RepositoryError(m) if m == "db down"));
    }
}
