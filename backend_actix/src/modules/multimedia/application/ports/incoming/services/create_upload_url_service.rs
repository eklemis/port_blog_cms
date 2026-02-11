use async_trait::async_trait;

use crate::multimedia::application::ports::{
    incoming::use_cases::{
        make_object_key, CreateAttachmentCommand, CreateMediaCommand, CreateMediaResult,
        CreateUploadMediaUrlUseCase, CreateUrlError,
    },
    outgoing::{
        cloud_storage::{MediaInfo, StorageQuery},
        db::{MediaRepository, RecordMediaError, RecordMediaTx},
    },
};

pub struct CreateUploadMediaUrlService<Q, R>
where
    Q: StorageQuery,
    R: MediaRepository,
{
    storage_query: Q,
    repository: R,
}

impl<Q, R> CreateUploadMediaUrlService<Q, R>
where
    Q: StorageQuery,
    R: MediaRepository,
{
    pub fn new(storage_query: Q, repository: R) -> Self {
        Self {
            storage_query,
            repository,
        }
    }
}

#[async_trait]
impl<Q, R> CreateUploadMediaUrlUseCase for CreateUploadMediaUrlService<Q, R>
where
    Q: StorageQuery + Send + Sync,
    R: MediaRepository + Send + Sync,
{
    async fn execute(
        &self,
        media_command: CreateMediaCommand,
        attachment_command: CreateAttachmentCommand,
    ) -> Result<CreateMediaResult, CreateUrlError> {
        // 1) Persist media + attachment atomically.
        let tx = RecordMediaTx {
            media: media_command.to_new_media(),
            attachment: attachment_command.to_new_attachment(),
        };

        let recorded = self
            .repository
            .record_media_tx(tx)
            .await
            .map_err(|err| match err {
                RecordMediaError::DatabaseError(e) => CreateUrlError::RepositoryError(e),
            })?;

        // 2) Generate safe object key (<uuid>.<ext>)
        let object_key = make_object_key(recorded.media_id, &recorded.original_name)
            .map_err(|e| CreateUrlError::RepositoryError(e.to_string()))?;

        // 3) Build validated MediaInfo (no unwrap)
        let media_info =
            MediaInfo::try_new(recorded.bucket_name, object_key, recorded.attachment_target)
                .map_err(|e| CreateUrlError::StorageError(e.to_string()))?;
        tracing::info!("media_info for signing: {:?}", &media_info);

        // 4) Ask storage adapter for signed upload URL (async).
        // IMPORTANT: Avoid `map_err(Into::into)` ambiguity by mapping explicitly.
        let url = self
            .storage_query
            .get_signed_upload_url(media_info)
            .await
            .map_err(CreateUrlError::from)?;

        Ok(CreateMediaResult {
            url,
            media_id: recorded.media_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    use crate::multimedia::application::{
        domain::{
            entities::{AttachmentTarget, MediaRole, MediaState, MediaStateInfo, MediaVariant},
            policies::upload_policy::UploadPolicy,
        },
        ports::outgoing::{
            cloud_storage::{
                ManifestInfo, MediaInfo, SignUrlError, StorageQuery, StorageQueryError,
            },
            db::{
                MediaRepository, MediaRepositoryError, MediaVariantRecord, RecordMediaError,
                RecordMediaTx, RecordedMedia, UpdateMediaStateData,
            },
        },
    };

    // ----------------------------
    // Mocks
    // ----------------------------

    #[derive(Clone)]
    struct MockRepo {
        result: Arc<Mutex<Result<RecordedMedia, RecordMediaError>>>,
        captured_tx: Arc<Mutex<Option<RecordMediaTx>>>,
    }

    impl MockRepo {
        fn new(result: Result<RecordedMedia, RecordMediaError>) -> Self {
            Self {
                result: Arc::new(Mutex::new(result)),
                captured_tx: Arc::new(Mutex::new(None)),
            }
        }

        fn captured(&self) -> Option<RecordMediaTx> {
            self.captured_tx.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl MediaRepository for MockRepo {
        async fn record_media_tx(
            &self,
            tx: RecordMediaTx,
        ) -> Result<RecordedMedia, RecordMediaError> {
            *self.captured_tx.lock().unwrap() = Some(tx);
            self.result.lock().unwrap().clone()
        }

        async fn set_media_state(
            &self,
            _data: UpdateMediaStateData,
        ) -> Result<MediaStateInfo, MediaRepositoryError> {
            Err(MediaRepositoryError::DatabaseError("not used".into()))
        }

        async fn record_single_variant(
            &self,
            _data: MediaVariantRecord,
        ) -> Result<MediaVariant, MediaRepositoryError> {
            Err(MediaRepositoryError::DatabaseError("not used".into()))
        }

        async fn record_variants(
            &self,
            _data: Vec<MediaVariantRecord>,
        ) -> Result<Vec<MediaVariant>, MediaRepositoryError> {
            Err(MediaRepositoryError::DatabaseError("not used".into()))
        }
    }

    #[derive(Clone)]
    struct MockStorage {
        result: Arc<Mutex<Result<String, SignUrlError>>>,
        captured_info: Arc<Mutex<Option<MediaInfo>>>,
    }

    impl MockStorage {
        fn new(result: Result<String, SignUrlError>) -> Self {
            Self {
                result: Arc::new(Mutex::new(result)),
                captured_info: Arc::new(Mutex::new(None)),
            }
        }

        fn captured(&self) -> Option<MediaInfo> {
            self.captured_info.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl StorageQuery for MockStorage {
        async fn get_signed_upload_url(
            &self,
            media_info: MediaInfo,
        ) -> Result<String, SignUrlError> {
            *self.captured_info.lock().unwrap() = Some(media_info);
            self.result.lock().unwrap().clone()
        }
        async fn get_signed_read_url(
            &self,
            _media_info: MediaInfo,
        ) -> Result<String, SignUrlError> {
            unimplemented!()
        }
        async fn get_latest_manifest(
            &self,
            _media_id: &str,
        ) -> Result<ManifestInfo, StorageQueryError> {
            Err(StorageQueryError::ManifestNotFound)
        }
    }

    // ----------------------------
    // Test helpers
    // ----------------------------

    fn policy_with_bucket(bucket: &str) -> UploadPolicy {
        // If your UploadPolicy doesn’t allow struct literal construction, adjust here.
        UploadPolicy {
            max_file_size_bytes: 5 * 1024 * 1024,
            max_width_height_px: 6000,
            max_file_name_len: 255,
            allowed_mime_types: &["image/jpeg", "image/png", "image/webp"],
            bucket_name: bucket.to_string(),
        }
    }

    fn dummy_user_id() -> crate::auth::application::domain::entities::UserId {
        // Adjust this if your UserId constructor differs.
        // Common patterns: UserId(Uuid), UserId::new(Uuid), Uuid::into(), etc.
        Uuid::new_v4().into()
    }

    fn dummy_attachment_target() -> AttachmentTarget {
        // Adjust to a valid variant for your enum if Default isn’t implemented.
        // e.g. AttachmentTarget::Post, AttachmentTarget::Article, etc.
        AttachmentTarget::default()
    }

    fn dummy_role() -> MediaRole {
        // Adjust to a valid variant if Default isn’t implemented.
        MediaRole::default()
    }

    fn build_valid_commands(bucket: &str) -> (CreateMediaCommand, CreateAttachmentCommand) {
        let policy = policy_with_bucket(bucket);

        let owner = dummy_user_id();
        let target = dummy_attachment_target();
        let role = dummy_role();

        let media = CreateMediaCommand::builder()
            .owner(owner)
            .file_name("cat.png".to_string())
            .mime_type("image/png".to_string())
            .file_size_bytes(1024)
            .width_px(Some(400))
            .height_px(Some(300))
            .build(&policy)
            .expect("valid media command");

        let attachment = CreateAttachmentCommand::builder()
            .owner(dummy_user_id()) // independent owner value if needed; change to same if required by your domain
            .attachment_target(target)
            .attachment_target_id(Uuid::new_v4())
            .role(role)
            .position(0)
            .build()
            .expect("valid attachment command");

        (media, attachment)
    }

    fn recorded_media(
        bucket: &str,
        original_name: &str,
        attachment_target: AttachmentTarget,
    ) -> RecordedMedia {
        RecordedMedia {
            owner: dummy_user_id(),
            media_id: Uuid::new_v4(),
            bucket_name: bucket.to_string(),
            original_name: original_name.to_string(),
            attachment_target,
            state: MediaState::Pending,
        }
    }

    // ----------------------------
    // Tests
    // ----------------------------

    #[tokio::test]
    async fn execute_success_returns_signed_url_and_passes_expected_media_info() {
        let expected_url = "https://signed.example/upload".to_string();

        let target = dummy_attachment_target();
        let repo = MockRepo::new(Ok(recorded_media("bucket-a", "cat.png", target.clone())));
        let storage = MockStorage::new(Ok(expected_url.clone()));

        let svc = CreateUploadMediaUrlService::new(storage.clone(), repo.clone());

        let (media_cmd, attachment_cmd) = build_valid_commands("bucket-a");
        let media_result = svc
            .execute(media_cmd, attachment_cmd)
            .await
            .expect("success");

        assert_eq!(media_result.url, expected_url);

        // Repo called with tx
        let tx = repo.captured().expect("repo should be called");
        assert_eq!(tx.media.bucket_name, "bucket-a");
        assert_eq!(tx.media.state, MediaState::Pending);

        // Storage called with MediaInfo
        let info = storage.captured().expect("storage should be called");
        assert_eq!(info.bucket_name(), "bucket-a");
        assert_eq!(info.attachment_target(), &target);

        // object_name should be "<uuid>.png" (generated from recorded media_id + ext)
        // We can’t know the uuid here easily, but we can ensure it ends with ".png"
        assert!(info.object_name().ends_with(".png"));
    }

    #[tokio::test]
    async fn execute_repo_error_maps_to_create_url_repository_error() {
        let repo = MockRepo::new(Err(RecordMediaError::DatabaseError("db down".into())));
        let storage = MockStorage::new(Ok("unused".into()));

        let svc = CreateUploadMediaUrlService::new(storage, repo);

        let (media_cmd, attachment_cmd) = build_valid_commands("bucket-a");
        let err = svc.execute(media_cmd, attachment_cmd).await.unwrap_err();

        match err {
            CreateUrlError::RepositoryError(msg) => assert!(msg.contains("db down")),
            other => panic!("expected RepositoryError, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn execute_make_object_key_error_maps_to_repository_error_and_skips_storage() {
        let target = dummy_attachment_target();

        // original_name without extension => make_object_key should fail
        let repo = MockRepo::new(Ok(recorded_media("bucket-a", "no_extension", target)));
        let storage = MockStorage::new(Ok("unused".into()));

        let svc = CreateUploadMediaUrlService::new(storage.clone(), repo);

        let (media_cmd, attachment_cmd) = build_valid_commands("bucket-a");
        let err = svc.execute(media_cmd, attachment_cmd).await.unwrap_err();

        match err {
            CreateUrlError::RepositoryError(_) => {}
            other => panic!("expected RepositoryError, got: {other:?}"),
        }

        // storage must not be called
        assert!(storage.captured().is_none());
    }

    #[tokio::test]
    async fn execute_media_info_validation_error_maps_to_storage_error_and_skips_storage() {
        let target = dummy_attachment_target();

        // empty bucket_name => MediaInfo::try_new should fail
        let repo = MockRepo::new(Ok(recorded_media("", "cat.png", target)));
        let storage = MockStorage::new(Ok("unused".into()));

        let svc = CreateUploadMediaUrlService::new(storage.clone(), repo);

        let (media_cmd, attachment_cmd) = build_valid_commands("bucket-a");
        let err = svc.execute(media_cmd, attachment_cmd).await.unwrap_err();

        match err {
            CreateUrlError::StorageError(msg) => {
                // exact message depends on MediaInfoError Display, but should be non-empty
                assert!(!msg.trim().is_empty());
            }
            other => panic!("expected StorageError, got: {other:?}"),
        }

        // storage must not be called
        assert!(storage.captured().is_none());
    }

    #[tokio::test]
    async fn execute_storage_error_maps_to_create_url_storage_error() {
        let target = dummy_attachment_target();

        let repo = MockRepo::new(Ok(recorded_media("bucket-a", "cat.png", target)));
        let storage = MockStorage::new(Err(SignUrlError::AccessDenied));

        let svc = CreateUploadMediaUrlService::new(storage, repo);

        let (media_cmd, attachment_cmd) = build_valid_commands("bucket-a");
        let err = svc.execute(media_cmd, attachment_cmd).await.unwrap_err();

        match err {
            CreateUrlError::StorageError(msg) => {
                // relies on SignUrlError Display; just ensure we got something meaningful
                assert!(!msg.trim().is_empty());
            }
            other => panic!("expected StorageError, got: {other:?}"),
        }
    }
}
