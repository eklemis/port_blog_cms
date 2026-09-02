//! Implementations of the media lifecycle contracts.
//!
//! All four are thin: the repository and query are already owner-scoped in SQL,
//! so these exist to map an outgoing error onto the error the endpoint speaks.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::domain::entities::AttachmentTarget;
use crate::multimedia::application::ports::incoming::use_cases::{
    GetMediaStatusesUseCase, GetMediaUsageUseCase, HardDeleteMediaUseCase, MediaLifecycleError,
    MediaStatus, MediaUsage, PatchMediaUseCase, RestoreMediaUseCase,
};
use crate::multimedia::application::ports::outgoing::db::{
    MediaQuery, MediaRepository, PatchAttachmentData,
};

/// Corrects an attachment's metadata.
#[derive(Clone)]
pub struct PatchMediaService<R> {
    repository: R,
}

/// Returns a soft-deleted item to service.
#[derive(Clone)]
pub struct RestoreMediaService<R> {
    repository: R,
}

/// Removes an item permanently.
#[derive(Clone)]
pub struct HardDeleteMediaService<R> {
    repository: R,
}

/// Reports where an item is used.
#[derive(Clone)]
pub struct GetMediaUsageService<Q> {
    query: Q,
}

macro_rules! repo_service {
    ($name:ident) => {
        impl<R> $name<R> {
            /// Builds it from the ports it depends on.
            pub fn new(repository: R) -> Self {
                Self { repository }
            }
        }
    };
}
repo_service!(PatchMediaService);
repo_service!(RestoreMediaService);
repo_service!(HardDeleteMediaService);

impl<Q> GetMediaUsageService<Q> {
    /// Builds it from the ports it depends on.
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<R: MediaRepository + Send + Sync> PatchMediaUseCase for PatchMediaService<R> {
    async fn execute(
        &self,
        owner: UserId,
        media_id: Uuid,
        data: PatchAttachmentData,
    ) -> Result<(), MediaLifecycleError> {
        self.repository
            .patch_attachment(owner, media_id, data)
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl<R: MediaRepository + Send + Sync> RestoreMediaUseCase for RestoreMediaService<R> {
    async fn execute(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaLifecycleError> {
        self.repository
            .restore(owner, media_id)
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl<R: MediaRepository + Send + Sync> HardDeleteMediaUseCase for HardDeleteMediaService<R> {
    async fn execute(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaLifecycleError> {
        self.repository
            .hard_delete(owner, media_id)
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl<Q: MediaQuery + Send + Sync> GetMediaUsageUseCase for GetMediaUsageService<Q> {
    async fn execute(
        &self,
        owner: UserId,
        media_id: Uuid,
    ) -> Result<Vec<MediaUsage>, MediaLifecycleError> {
        let rows = self
            .query
            .find_media_usage(owner, media_id)
            .await
            .map_err(|e| MediaLifecycleError::RepositoryError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| MediaUsage {
                // An unrecognised stored value is reported as `User` rather
                // than dropping the row: a delete confirmation that silently
                // omits a usage is worse than one naming it imprecisely.
                target: match r.attachable_type.as_str() {
                    "blog_post" => AttachmentTarget::BlogPost,
                    "project" => AttachmentTarget::Project,
                    "resume" => AttachmentTarget::Resume,
                    _ => AttachmentTarget::User,
                },
                target_id: r.attachable_id,
                role: r.role,
                is_published: r.is_published,
            })
            .collect())
    }
}

/// Reports several items' processing states at once.
#[derive(Clone)]
pub struct GetMediaStatusesService<Q> {
    query: Q,
}

impl<Q> GetMediaStatusesService<Q> {
    /// Builds it from the ports it depends on.
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q: MediaQuery + Send + Sync> GetMediaStatusesUseCase for GetMediaStatusesService<Q> {
    async fn execute(
        &self,
        owner: UserId,
        media_ids: Vec<Uuid>,
    ) -> Result<Vec<MediaStatus>, MediaLifecycleError> {
        let rows = self
            .query
            .get_states(owner, &media_ids)
            .await
            .map_err(|e| MediaLifecycleError::RepositoryError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|s| MediaStatus {
                media_id: s.media_id,
                state: s.status,
                updated_at: s.updated_at,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multimedia::application::domain::entities::{MediaSize, MediaStateInfo};
    use crate::multimedia::application::ports::outgoing::db::{
        MediaAttachment, MediaQueryError, MediaRepositoryError, MediaUsageRow, StoredVariant,
    };
    use std::sync::Mutex;

    struct StubQuery(Vec<MediaUsageRow>);

    #[async_trait]
    impl MediaQuery for StubQuery {
        async fn get_states(
            &self,
            _owner: UserId,
            _media_ids: &[Uuid],
        ) -> Result<Vec<MediaStateInfo>, MediaQueryError> {
            Ok(Vec::new())
        }

        async fn find_media_usage(
            &self,
            _o: UserId,
            _m: Uuid,
        ) -> Result<Vec<MediaUsageRow>, MediaQueryError> {
            Ok(self.0.clone())
        }
        async fn find_public_variant(
            &self,
            _m: Uuid,
            _s: MediaSize,
        ) -> Result<Option<StoredVariant>, MediaQueryError> {
            unimplemented!()
        }
        async fn get_state(&self, _m: Uuid) -> Result<MediaStateInfo, MediaQueryError> {
            unimplemented!()
        }
        async fn list_by_target(
            &self,
            _o: UserId,
            _t: AttachmentTarget,
        ) -> Result<Vec<MediaAttachment>, MediaQueryError> {
            unimplemented!()
        }
        async fn get_attachment_info(&self, _m: Uuid) -> Result<MediaAttachment, MediaQueryError> {
            unimplemented!()
        }
    }

    fn row(kind: &str, published: bool) -> MediaUsageRow {
        MediaUsageRow {
            attachable_type: kind.into(),
            attachable_id: Uuid::new_v4(),
            role: "cover".into(),
            is_published: published,
        }
    }

    #[tokio::test]
    async fn usage_maps_stored_target_types_onto_the_domain_enum() {
        let svc = GetMediaUsageService::new(StubQuery(vec![
            row("blog_post", true),
            row("project", true),
            row("resume", false),
        ]));

        let usage = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap();

        let targets: Vec<_> = usage.iter().map(|u| u.target.clone()).collect();
        assert_eq!(
            targets,
            vec![
                AttachmentTarget::BlogPost,
                AttachmentTarget::Project,
                AttachmentTarget::Resume
            ]
        );
    }

    /// `is_published` is the field the endpoint exists for, so it must survive
    /// the mapping untouched rather than being recomputed or defaulted.
    #[tokio::test]
    async fn the_published_flag_is_carried_through_unchanged() {
        let svc = GetMediaUsageService::new(StubQuery(vec![
            row("blog_post", true),
            row("blog_post", false),
        ]));

        let usage = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap();

        assert_eq!(
            usage.iter().map(|u| u.is_published).collect::<Vec<_>>(),
            vec![true, false]
        );
    }

    /// An unrecognised stored value must not silently drop the row: a delete
    /// confirmation that omits a usage is more dangerous than one naming it
    /// imprecisely.
    #[tokio::test]
    async fn an_unknown_target_type_is_still_reported() {
        let svc = GetMediaUsageService::new(StubQuery(vec![row("something_new", true)]));

        let usage = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap();

        assert_eq!(usage.len(), 1, "the row must not be dropped");
    }

    #[tokio::test]
    async fn an_unused_item_reports_an_empty_list_rather_than_not_found() {
        let svc = GetMediaUsageService::new(StubQuery(Vec::new()));

        let usage = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap();

        assert!(usage.is_empty());
    }

    #[derive(Clone)]
    struct StubRepo {
        called: std::sync::Arc<Mutex<Vec<&'static str>>>,
        result: Result<(), MediaRepositoryError>,
    }

    impl StubRepo {
        fn new(result: Result<(), MediaRepositoryError>) -> Self {
            Self {
                called: std::sync::Arc::new(Mutex::new(Vec::new())),
                result,
            }
        }
        fn calls(&self) -> Vec<&'static str> {
            self.called.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl MediaRepository for StubRepo {
        async fn patch_attachment(
            &self,
            _o: UserId,
            _m: Uuid,
            _d: PatchAttachmentData,
        ) -> Result<(), MediaRepositoryError> {
            self.called.lock().unwrap().push("patch");
            self.result.clone()
        }
        async fn restore(&self, _o: UserId, _m: Uuid) -> Result<(), MediaRepositoryError> {
            self.called.lock().unwrap().push("restore");
            self.result.clone()
        }
        async fn hard_delete(&self, _o: UserId, _m: Uuid) -> Result<(), MediaRepositoryError> {
            self.called.lock().unwrap().push("hard_delete");
            self.result.clone()
        }
        async fn soft_delete(&self, _o: UserId, _m: Uuid) -> Result<(), MediaRepositoryError> {
            self.called.lock().unwrap().push("soft_delete");
            self.result.clone()
        }
        async fn record_media_tx(
            &self,
            _tx: crate::multimedia::application::ports::outgoing::db::RecordMediaTx,
        ) -> Result<
            crate::multimedia::application::ports::outgoing::db::RecordedMedia,
            crate::multimedia::application::ports::outgoing::db::RecordMediaError,
        > {
            unimplemented!()
        }
        async fn set_media_state(
            &self,
            _d: crate::multimedia::application::ports::outgoing::db::UpdateMediaStateData,
        ) -> Result<MediaStateInfo, MediaRepositoryError> {
            unimplemented!()
        }
        async fn record_single_variant(
            &self,
            _d: crate::multimedia::application::ports::outgoing::db::MediaVariantRecord,
        ) -> Result<
            crate::multimedia::application::domain::entities::MediaVariant,
            MediaRepositoryError,
        > {
            unimplemented!()
        }
        async fn record_variants(
            &self,
            _d: Vec<crate::multimedia::application::ports::outgoing::db::MediaVariantRecord>,
        ) -> Result<
            Vec<crate::multimedia::application::domain::entities::MediaVariant>,
            MediaRepositoryError,
        > {
            unimplemented!()
        }
    }

    /// Hard delete must not route to soft delete. Getting these two crossed
    /// would make "delete permanently" reversible or "delete" permanent, and
    /// the wiring is a one-word difference.
    #[tokio::test]
    async fn each_lifecycle_service_calls_its_own_repository_method() {
        let owner = UserId::from(Uuid::new_v4());
        let id = Uuid::new_v4();

        let r = StubRepo::new(Ok(()));
        PatchMediaService::new(r.clone())
            .execute(owner, id, PatchAttachmentData::default())
            .await
            .unwrap();
        assert_eq!(r.calls(), vec!["patch"]);

        let r = StubRepo::new(Ok(()));
        RestoreMediaService::new(r.clone())
            .execute(owner, id)
            .await
            .unwrap();
        assert_eq!(r.calls(), vec!["restore"]);

        let r = StubRepo::new(Ok(()));
        HardDeleteMediaService::new(r.clone())
            .execute(owner, id)
            .await
            .unwrap();
        assert_eq!(
            r.calls(),
            vec!["hard_delete"],
            "hard delete must not route to soft delete: the wiring is a one-word difference"
        );
    }

    #[tokio::test]
    async fn a_missing_row_becomes_not_found_rather_than_a_500() {
        let r = StubRepo::new(Err(MediaRepositoryError::NotFound));

        let err = RestoreMediaService::new(r)
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();

        assert!(matches!(err, MediaLifecycleError::NotFound));
    }
}
