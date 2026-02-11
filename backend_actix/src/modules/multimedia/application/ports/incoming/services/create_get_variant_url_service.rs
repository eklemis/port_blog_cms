use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::multimedia::application::{
    domain::entities::MediaState,
    ports::{
        incoming::use_cases::{
            GetReadUrlError, GetUrlCommand, GetUrlResult, GetVariantReadUrlUseCase,
        },
        outgoing::{
            cloud_storage::{MediaInfo, StorageQuery},
            db::MediaQuery,
        },
    },
};

/// TTL for signed read URLs (15 minutes)
const SIGNED_URL_TTL_MINUTES: i64 = 15;

pub struct GetVariantReadUrlService<S, M>
where
    S: StorageQuery,
    M: MediaQuery,
{
    storage_query: S,
    media_query: M,
}

impl<S, M> GetVariantReadUrlService<S, M>
where
    S: StorageQuery,
    M: MediaQuery,
{
    pub fn new(storage_query: S, media_query: M) -> Self {
        Self {
            storage_query,
            media_query,
        }
    }

    fn map_query_error(
        err: crate::multimedia::application::ports::outgoing::db::MediaQueryError,
    ) -> GetReadUrlError {
        use crate::multimedia::application::ports::outgoing::db::MediaQueryError;

        match err {
            MediaQueryError::MediaNotFound => GetReadUrlError::MediaNotFound,
            MediaQueryError::DatabaseError(e) => GetReadUrlError::QueryError(e),
        }
    }

    fn map_storage_error(
        err: crate::multimedia::application::ports::outgoing::cloud_storage::SignUrlError,
    ) -> GetReadUrlError {
        use crate::multimedia::application::ports::outgoing::cloud_storage::SignUrlError;

        match err {
            SignUrlError::AccessDenied => {
                GetReadUrlError::StorageError("Access denied".to_string())
            }
            SignUrlError::BucketNotFound => {
                GetReadUrlError::StorageError("Bucket not found".to_string())
            }
            SignUrlError::Configuration => {
                GetReadUrlError::StorageError("Storage configuration error".to_string())
            }
            SignUrlError::Infrastructure => {
                GetReadUrlError::StorageError("Storage infrastructure error".to_string())
            }
        }
    }
}

#[async_trait]
impl<S, M> GetVariantReadUrlUseCase for GetVariantReadUrlService<S, M>
where
    S: StorageQuery,
    M: MediaQuery,
{
    async fn execute(&self, command: GetUrlCommand) -> Result<GetUrlResult, GetReadUrlError> {
        // 1. Get media attachment info
        let media = self
            .media_query
            .get_attachment_info(command.media_id)
            .await
            .map_err(Self::map_query_error)?;

        // 2. Verify ownership
        if media.owner != command.owner {
            return Err(GetReadUrlError::MediaNotFound);
        }

        // 3. Check media state
        match media.status {
            MediaState::Pending => return Err(GetReadUrlError::MediaPending),
            MediaState::Processing => return Err(GetReadUrlError::MediaProcessing),
            MediaState::Failed => return Err(GetReadUrlError::MediaFailed),
            MediaState::Ready => {}
        }

        // 4. Find the requested variant
        let variant = media
            .variants
            .iter()
            .find(|v| v.size == command.size)
            .ok_or_else(|| GetReadUrlError::VariantNotFound(command.size.clone()))?;

        // 5. Create MediaInfo for signing
        let media_info = MediaInfo::try_new(
            variant.bucket_name.clone(),
            variant.object_name.clone(),
            media.attachment_target,
        )
        .map_err(|e| GetReadUrlError::StorageError(e.to_string()))?;

        // 6. Get signed URL
        let url = self
            .storage_query
            .get_signed_read_url(media_info)
            .await
            .map_err(Self::map_storage_error)?;

        // 7. Calculate expiration time
        let expires_at = Utc::now() + Duration::minutes(SIGNED_URL_TTL_MINUTES);

        Ok(GetUrlResult {
            media_id: command.media_id,
            size: command.size,
            url,
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        auth::application::domain::entities::UserId,
        multimedia::application::{
            domain::entities::{
                AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo,
            },
            ports::outgoing::{
                cloud_storage::{ManifestInfo, SignUrlError, StorageQueryError},
                db::{MediaAttachment, MediaQueryError, StoredVariant},
            },
        },
    };

    // Mock MediaQuery
    struct MockMediaQuery {
        result: Result<MediaAttachment, MediaQueryError>,
    }

    #[async_trait]
    impl MediaQuery for MockMediaQuery {
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

    // Mock StorageQuery
    struct MockStorageQuery {
        result: Result<String, SignUrlError>,
    }

    #[async_trait]
    impl StorageQuery for MockStorageQuery {
        async fn get_signed_upload_url(
            &self,
            _media_info: MediaInfo,
        ) -> Result<String, SignUrlError> {
            unimplemented!()
        }

        async fn get_signed_read_url(
            &self,
            _media_info: MediaInfo,
        ) -> Result<String, SignUrlError> {
            self.result.clone()
        }

        async fn get_latest_manifest(
            &self,
            _media_id: &str,
        ) -> Result<ManifestInfo, StorageQueryError> {
            unimplemented!()
        }
    }

    fn create_test_media_attachment(
        owner: UserId,
        status: MediaState,
        variants: Vec<StoredVariant>,
    ) -> MediaAttachment {
        MediaAttachment {
            media_id: Uuid::new_v4(),
            owner,
            attachment_target: AttachmentTarget::Resume,
            attachment_target_id: Uuid::new_v4(),
            status,
            role: MediaRole::Profile,
            position: 0,
            alt_text: String::new(),
            caption: String::new(),
            original_filename: "test.jpg".to_string(),
            variants,
        }
    }

    fn create_test_variant(size: MediaSize) -> StoredVariant {
        StoredVariant {
            size,
            bucket_name: "test-bucket".to_string(),
            object_name: "test-object".to_string(),
            width: 800,
            height: 600,
            file_size_bytes: 1024,
            mime_type: "image/jpeg".to_string(),
        }
    }

    #[tokio::test]
    async fn execute_success() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();
        let size = MediaSize::Medium;

        let variant = create_test_variant(size.clone());
        let media = create_test_media_attachment(owner, MediaState::Ready, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Ok("https://signed-url.example.com".to_string()),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner,
            media_id,
            size: size.clone(),
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(result.is_ok());
        let url_result = result.unwrap();
        assert_eq!(url_result.media_id, media_id);
        assert_eq!(url_result.size, size);
        assert_eq!(url_result.url, "https://signed-url.example.com");
        assert!(url_result.expires_at > Utc::now());
    }

    #[tokio::test]
    async fn execute_media_not_found() {
        // Arrange
        let media_query = MockMediaQuery {
            result: Err(MediaQueryError::MediaNotFound),
        };
        let storage_query = MockStorageQuery {
            result: Ok("url".to_string()),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner: UserId::from(Uuid::new_v4()),
            media_id: Uuid::new_v4(),
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::MediaNotFound)));
    }

    #[tokio::test]
    async fn execute_database_error() {
        // Arrange
        let media_query = MockMediaQuery {
            result: Err(MediaQueryError::DatabaseError("DB error".to_string())),
        };
        let storage_query = MockStorageQuery {
            result: Ok("url".to_string()),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner: UserId::from(Uuid::new_v4()),
            media_id: Uuid::new_v4(),
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::QueryError(_))));
    }

    #[tokio::test]
    async fn execute_wrong_owner() {
        // Arrange
        let real_owner = UserId::from(Uuid::new_v4());
        let wrong_owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let variant = create_test_variant(MediaSize::Medium);
        let media = create_test_media_attachment(real_owner, MediaState::Ready, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Ok("url".to_string()),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner: wrong_owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::MediaNotFound)));
    }

    #[tokio::test]
    async fn execute_media_pending() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let variant = create_test_variant(MediaSize::Medium);
        let media = create_test_media_attachment(owner, MediaState::Pending, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Ok("url".to_string()),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::MediaPending)));
    }

    #[tokio::test]
    async fn execute_media_processing() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let variant = create_test_variant(MediaSize::Medium);
        let media = create_test_media_attachment(owner, MediaState::Processing, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Ok("url".to_string()),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::MediaProcessing)));
    }

    #[tokio::test]
    async fn execute_media_failed() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let variant = create_test_variant(MediaSize::Medium);
        let media = create_test_media_attachment(owner, MediaState::Failed, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Ok("url".to_string()),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::MediaFailed)));
    }

    #[tokio::test]
    async fn execute_variant_not_found() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        // Media has only Small variant
        let variant = create_test_variant(MediaSize::Small);
        let media = create_test_media_attachment(owner, MediaState::Ready, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Ok("url".to_string()),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        // Request Medium variant
        let command = GetUrlCommand {
            owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::VariantNotFound(_))));
    }

    #[tokio::test]
    async fn execute_storage_access_denied() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let variant = create_test_variant(MediaSize::Medium);
        let media = create_test_media_attachment(owner, MediaState::Ready, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Err(SignUrlError::AccessDenied),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::StorageError(_))));
        if let Err(GetReadUrlError::StorageError(msg)) = result {
            assert_eq!(msg, "Access denied");
        }
    }

    #[tokio::test]
    async fn execute_storage_bucket_not_found() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let variant = create_test_variant(MediaSize::Medium);
        let media = create_test_media_attachment(owner, MediaState::Ready, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Err(SignUrlError::BucketNotFound),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::StorageError(_))));
        if let Err(GetReadUrlError::StorageError(msg)) = result {
            assert_eq!(msg, "Bucket not found");
        }
    }

    #[tokio::test]
    async fn execute_storage_configuration_error() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let variant = create_test_variant(MediaSize::Medium);
        let media = create_test_media_attachment(owner, MediaState::Ready, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Err(SignUrlError::Configuration),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::StorageError(_))));
        if let Err(GetReadUrlError::StorageError(msg)) = result {
            assert_eq!(msg, "Storage configuration error");
        }
    }

    #[tokio::test]
    async fn execute_storage_infrastructure_error() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let variant = create_test_variant(MediaSize::Medium);
        let media = create_test_media_attachment(owner, MediaState::Ready, vec![variant]);

        let media_query = MockMediaQuery { result: Ok(media) };
        let storage_query = MockStorageQuery {
            result: Err(SignUrlError::Infrastructure),
        };

        let service = GetVariantReadUrlService::new(storage_query, media_query);

        let command = GetUrlCommand {
            owner,
            media_id,
            size: MediaSize::Medium,
        };

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(matches!(result, Err(GetReadUrlError::StorageError(_))));
        if let Err(GetReadUrlError::StorageError(msg)) = result {
            assert_eq!(msg, "Storage infrastructure error");
        }
    }
}
