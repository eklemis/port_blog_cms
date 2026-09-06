//! Implements the sweep that removes registrations whose bytes never arrived.

use async_trait::async_trait;
use std::sync::Arc;

use crate::multimedia::application::ports::{
    incoming::use_cases::{
        ReapError, ReapOutcome, ReapStaleUploadsUseCase, DEFAULT_STALE_AFTER_SECS,
        MIN_STALE_AFTER_SECS,
    },
    outgoing::db::MediaRepository,
};

/// Implements the corresponding use-case contract.
pub struct ReapStaleUploadsService {
    repo: Arc<dyn MediaRepository>,
    configured: u64,
}

impl ReapStaleUploadsService {
    /// Builds it from the ports it depends on.
    ///
    /// `configured` is the deployment's window; `None` takes the default.
    pub fn new(repo: Arc<dyn MediaRepository>, configured: Option<u64>) -> Self {
        Self {
            repo,
            configured: configured.unwrap_or(DEFAULT_STALE_AFTER_SECS),
        }
    }
}

#[async_trait]
impl ReapStaleUploadsUseCase for ReapStaleUploadsService {
    async fn execute(&self, older_than_secs: Option<u64>) -> Result<ReapOutcome, ReapError> {
        // The floor is enforced rather than trusted. The window decides whether
        // a row is dead or merely young, and the cost of the two mistakes is
        // not symmetric: sweeping too late leaves a stale row nobody sees,
        // while sweeping too early deletes a file somebody successfully
        // uploaded moments ago. A caller asking for a shorter window gets the
        // floor, and the response says which window ran.
        let requested = older_than_secs.unwrap_or(self.configured);
        let older_than_secs = requested.max(MIN_STALE_AFTER_SECS);

        let deleted = self
            .repo
            .delete_stale_pending(older_than_secs)
            .await
            .map_err(|e| ReapError::RepositoryError(e.to_string()))?;

        if deleted > 0 {
            tracing::info!(
                deleted,
                older_than_secs,
                "Removed uploads that were registered but never arrived"
            );
        }

        Ok(ReapOutcome {
            deleted,
            older_than_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use crate::multimedia::application::domain::entities::{MediaStateInfo, MediaVariant};
    use crate::multimedia::application::ports::outgoing::db::{
        MediaRepositoryError, MediaVariantRecord, PatchAttachmentData, RecordMediaError,
        RecordMediaTx, RecordedMedia, UpdateMediaStateData,
    };
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    struct FakeRepo {
        seen: Mutex<Vec<u64>>,
        deleted: u64,
        fail: bool,
    }

    #[async_trait]
    impl MediaRepository for FakeRepo {
        async fn delete_stale_pending(&self, s: u64) -> Result<u64, MediaRepositoryError> {
            self.seen.lock().unwrap().push(s);
            if self.fail {
                return Err(MediaRepositoryError::DatabaseError("down".into()));
            }
            Ok(self.deleted)
        }

        async fn patch_attachment(
            &self,
            _o: UserId,
            _m: Uuid,
            _d: PatchAttachmentData,
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
            _d: UpdateMediaStateData,
        ) -> Result<MediaStateInfo, MediaRepositoryError> {
            unimplemented!()
        }
        async fn record_single_variant(
            &self,
            _d: MediaVariantRecord,
        ) -> Result<MediaVariant, MediaRepositoryError> {
            unimplemented!()
        }
        async fn record_variants(
            &self,
            _d: Vec<MediaVariantRecord>,
        ) -> Result<Vec<MediaVariant>, MediaRepositoryError> {
            unimplemented!()
        }
        async fn soft_delete(&self, _o: UserId, _m: Uuid) -> Result<(), MediaRepositoryError> {
            unimplemented!()
        }
    }

    fn service(repo: Arc<FakeRepo>, configured: Option<u64>) -> ReapStaleUploadsService {
        ReapStaleUploadsService::new(repo, configured)
    }

    #[tokio::test]
    async fn it_reports_what_it_removed() {
        let repo = Arc::new(FakeRepo {
            deleted: 7,
            ..Default::default()
        });

        let out = service(Arc::clone(&repo), None)
            .execute(None)
            .await
            .unwrap();

        assert_eq!(out.deleted, 7);
        assert_eq!(out.older_than_secs, DEFAULT_STALE_AFTER_SECS);
    }

    /// The mistake this guard exists for. A window shorter than the signed
    /// upload URL's own lifetime would delete rows whose upload is still valid
    /// and may be in flight — taking a file somebody successfully uploaded.
    #[tokio::test]
    async fn a_window_shorter_than_the_upload_url_lifetime_is_raised_to_the_floor() {
        let repo = Arc::new(FakeRepo::default());

        let out = service(Arc::clone(&repo), None)
            .execute(Some(30))
            .await
            .unwrap();

        assert_eq!(repo.seen.lock().unwrap()[0], MIN_STALE_AFTER_SECS);
        assert_eq!(
            out.older_than_secs, MIN_STALE_AFTER_SECS,
            "the response must say which window actually ran"
        );
    }

    /// Including when the deployment itself is misconfigured, not just when a
    /// caller asks.
    #[tokio::test]
    async fn a_misconfigured_deployment_window_is_also_raised() {
        let repo = Arc::new(FakeRepo::default());

        service(Arc::clone(&repo), Some(60))
            .execute(None)
            .await
            .unwrap();

        assert_eq!(repo.seen.lock().unwrap()[0], MIN_STALE_AFTER_SECS);
    }

    #[tokio::test]
    async fn a_longer_window_is_honoured() {
        let repo = Arc::new(FakeRepo::default());

        service(Arc::clone(&repo), None)
            .execute(Some(86_400))
            .await
            .unwrap();

        assert_eq!(repo.seen.lock().unwrap()[0], 86_400);
    }

    /// A scheduler retries, so a sweep that finds nothing must be ordinary.
    #[tokio::test]
    async fn finding_nothing_is_success_not_an_error() {
        let repo = Arc::new(FakeRepo::default());

        let out = service(repo, None).execute(None).await.unwrap();

        assert_eq!(out.deleted, 0);
    }

    #[tokio::test]
    async fn a_failing_store_is_reported() {
        let repo = Arc::new(FakeRepo {
            fail: true,
            ..Default::default()
        });

        let err = service(repo, None).execute(None).await.unwrap_err();

        assert!(matches!(err, ReapError::RepositoryError(_)));
    }
}
