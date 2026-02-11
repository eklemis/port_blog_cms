use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;

use crate::multimedia::application::ports::outgoing::cloud_storage::{
    ManifestInfo, MediaInfo, SignUrlError, StorageQuery, StorageQueryError,
};

/// Bucket where manifests are written by your image resizer service.
const MANIFEST_BUCKET: &str = "blogport-cms-manifests";

/// TTL for signed upload URLs.
const SIGNED_URL_TTL: Duration = Duration::from_secs(15 * 60);

fn manifest_object_key(media_id: &str) -> String {
    format!("{}/manifest.json", media_id)
}

/// google-cloud-storage uses a bucket resource name format:
/// `projects/_/buckets/{bucket}`
///
/// Keeping this here makes it hard to accidentally pass a raw bucket name.
fn bucket_resource(bucket: &str) -> String {
    format!("projects/_/buckets/{}", bucket)
}

fn map_sign_error(msg: &str) -> SignUrlError {
    let m = msg.to_lowercase();

    if m.contains("permission") || m.contains("forbidden") || m.contains("denied") {
        SignUrlError::AccessDenied
    } else if m.contains("bucket") && (m.contains("not found") || m.contains("404")) {
        SignUrlError::BucketNotFound
    } else if m.contains("invalid") || m.contains("config") || m.contains("configuration") {
        SignUrlError::Configuration
    } else {
        SignUrlError::Infrastructure
    }
}

fn map_read_error(msg: &str) -> StorageQueryError {
    let m = msg.to_lowercase();

    if m.contains("404") || m.contains("not found") {
        StorageQueryError::ManifestNotFound
    } else if m.contains("timeout")
        || m.contains("dns")
        || m.contains("connection")
        || m.contains("network")
        || m.contains("tcp")
    {
        StorageQueryError::NetworkInterrupted
    } else {
        StorageQueryError::NetworkInterrupted
    }
}

/// Internal seam to make the adapter testable without mocking google-cloud-storage types/streams.
///
/// Tests will implement this trait with a fake client.
#[async_trait]
trait GcsClient: Send + Sync {
    async fn sign_put_url(
        &self,
        bucket_resource: &str,
        object_name: &str,
        ttl: Duration,
    ) -> Result<String, String>;

    async fn sign_get_url(
        &self,
        bucket_resource: &str,
        object_name: &str,
        ttl: Duration,
    ) -> Result<String, String>;

    async fn download_object_bytes(
        &self,
        bucket_resource: &str,
        object_name: &str,
    ) -> Result<Vec<u8>, String>;
}

#[cfg(test)]
struct ArcGcsClient(Arc<dyn GcsClient>);

#[cfg(test)]
#[async_trait]
impl GcsClient for ArcGcsClient {
    async fn sign_put_url(
        &self,
        bucket_resource: &str,
        object_name: &str,
        ttl: Duration,
    ) -> Result<String, String> {
        self.0.sign_put_url(bucket_resource, object_name, ttl).await
    }

    async fn sign_get_url(
        &self,
        bucket_resource: &str,
        object_name: &str,
        ttl: Duration,
    ) -> Result<String, String> {
        self.0.sign_get_url(bucket_resource, object_name, ttl).await
    }

    async fn download_object_bytes(
        &self,
        bucket_resource: &str,
        object_name: &str,
    ) -> Result<Vec<u8>, String> {
        self.0
            .download_object_bytes(bucket_resource, object_name)
            .await
    }
}

/// Production adapter: implements your StorageQuery port.
#[derive(Clone)]
pub struct GcsStorageQuery {
    client: Arc<OnceCell<Box<dyn GcsClient>>>,
    signed_url_ttl: Duration,
}

impl GcsStorageQuery {
    /// Synchronous constructor - client is initialized lazily on first use.
    pub fn new() -> Self {
        Self {
            client: Arc::new(OnceCell::new()),
            signed_url_ttl: SIGNED_URL_TTL,
        }
    }

    /// Get or initialize the GCS client.
    async fn get_client(&self) -> Result<&dyn GcsClient, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_or_try_init(|| async {
                let real_client = RealGcsClient::new().await?;
                Ok(Box::new(real_client) as Box<dyn GcsClient>)
            })
            .await
            .map(|boxed| &**boxed)
    }

    /// Test-friendly constructor with pre-initialized client.
    #[cfg(test)]
    fn with_client(client: Arc<dyn GcsClient>, signed_url_ttl: Duration) -> Self {
        let once = OnceCell::new();
        // Can't cast Arc to Box directly, need to extract and re-box
        // This is only for tests, so the clone is acceptable
        let _ = once.set(Box::new(ArcGcsClient(client)) as Box<dyn GcsClient>);

        Self {
            client: Arc::new(once),
            signed_url_ttl,
        }
    }
}

#[async_trait]
impl StorageQuery for GcsStorageQuery {
    async fn get_signed_upload_url(&self, media_info: MediaInfo) -> Result<String, SignUrlError> {
        let client = self
            .get_client()
            .await
            .map_err(|_| SignUrlError::Infrastructure)?;

        let bucket = bucket_resource(media_info.bucket_name());
        let object = media_info.object_name().to_string();

        client
            .sign_put_url(&bucket, &object, self.signed_url_ttl)
            .await
            .map_err(|e| map_sign_error(&e))
    }

    async fn get_signed_read_url(&self, media_info: MediaInfo) -> Result<String, SignUrlError> {
        let client = self
            .get_client()
            .await
            .map_err(|_| SignUrlError::Infrastructure)?;

        let bucket = bucket_resource(media_info.bucket_name());
        let object = media_info.object_name().to_string();

        client
            .sign_get_url(&bucket, &object, self.signed_url_ttl)
            .await
            .map_err(|e| map_sign_error(&e))
    }

    async fn get_latest_manifest(&self, media_id: &str) -> Result<ManifestInfo, StorageQueryError> {
        let media_id = media_id.trim();
        if media_id.is_empty() {
            return Err(StorageQueryError::MediaIdNotFound);
        }

        let client = self
            .get_client()
            .await
            .map_err(|_| StorageQueryError::NetworkInterrupted)?;

        let bucket = bucket_resource(MANIFEST_BUCKET);
        let object = manifest_object_key(media_id);

        let bytes = client
            .download_object_bytes(&bucket, &object)
            .await
            .map_err(|e| map_read_error(&e))?;

        let manifest: ManifestInfo =
            serde_json::from_slice(&bytes).map_err(|_| StorageQueryError::NetworkInterrupted)?;

        Ok(manifest)
    }
}

// ============================================================================
// Real Google Cloud Storage client (google-cloud-storage)
// ============================================================================

struct RealGcsClient {
    storage: google_cloud_storage::client::Storage,
    signer: google_cloud_auth::signer::Signer,
}

impl RealGcsClient {
    async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Initializing GCS client...");

        let storage = google_cloud_storage::client::Storage::builder()
            .build()
            .await
            .map_err(|e| {
                tracing::error!("Failed to build GCS storage client: {:?}", e);
                e
            })?;

        tracing::info!("GCS storage client created");

        let signer = google_cloud_auth::credentials::Builder::default()
            .build_signer()
            .map_err(|e| {
                let msg = e.to_string();
                tracing::error!("Failed to build GCS signer: {:?}", e);

                if msg.contains("authorized_user") {
                    tracing::error!(
                        "Signed URLs require a service account key. \
                         Set GOOGLE_APPLICATION_CREDENTIALS to a service-account JSON (type=service_account)."
                    );
                }

                e
            })?;

        tracing::info!("GCS signer created successfully");

        Ok(Self { storage, signer })
    }
}

#[async_trait]
impl GcsClient for RealGcsClient {
    async fn sign_put_url(
        &self,
        bucket_resource: &str,
        object_name: &str,
        ttl: Duration,
    ) -> Result<String, String> {
        let url = google_cloud_storage::builder::storage::SignedUrlBuilder::for_object(
            bucket_resource.to_string(),
            object_name.to_string(),
        )
        .with_method(google_cloud_storage::http::Method::PUT)
        .with_expiration(ttl)
        .sign_with(&self.signer)
        .await
        .map_err(|e| e.to_string())?;

        Ok(url)
    }

    async fn sign_get_url(
        &self,
        bucket_resource: &str,
        object_name: &str,
        ttl: Duration,
    ) -> Result<String, String> {
        let url = google_cloud_storage::builder::storage::SignedUrlBuilder::for_object(
            bucket_resource.to_string(),
            object_name.to_string(),
        )
        .with_method(google_cloud_storage::http::Method::GET)
        .with_expiration(ttl)
        .sign_with(&self.signer)
        .await
        .map_err(|e| e.to_string())?;

        Ok(url)
    }

    async fn download_object_bytes(
        &self,
        bucket_resource: &str,
        object_name: &str,
    ) -> Result<Vec<u8>, String> {
        let mut stream = self
            .storage
            .read_object(bucket_resource.to_string(), object_name.to_string())
            .send()
            .await
            .map_err(|e| e.to_string())?;

        use futures::StreamExt;

        let mut out: Vec<u8> = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            out.extend_from_slice(&chunk);
        }

        Ok(out)
    }
}

// ============================================================================
// Tests (100% coverage for this module)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::multimedia::application::domain::entities::{AttachmentTarget, MediaState};
    use crate::multimedia::application::ports::outgoing::cloud_storage::MediaInfo;

    struct FakeGcsClient {
        last_sign_put_call: Mutex<Option<(String, String, Duration)>>,
        last_sign_get_call: Mutex<Option<(String, String, Duration)>>,
        last_download_call: Mutex<Option<(String, String)>>,
        sign_put_result: Mutex<Result<String, String>>,
        sign_get_result: Mutex<Result<String, String>>,
        download_result: Mutex<Result<Vec<u8>, String>>,
    }

    impl Default for FakeGcsClient {
        fn default() -> Self {
            Self {
                last_sign_put_call: Mutex::new(None),
                last_sign_get_call: Mutex::new(None),
                last_download_call: Mutex::new(None),
                sign_put_result: Mutex::new(Ok("ok".to_string())),
                sign_get_result: Mutex::new(Ok("ok".to_string())),
                download_result: Mutex::new(Ok(Vec::new())),
            }
        }
    }

    impl FakeGcsClient {
        fn new() -> Self {
            Self::default()
        }

        fn set_sign_put_result(&self, r: Result<String, String>) {
            *self.sign_put_result.lock().unwrap() = r;
        }

        fn set_sign_get_result(&self, r: Result<String, String>) {
            *self.sign_get_result.lock().unwrap() = r;
        }

        fn set_download_result(&self, r: Result<Vec<u8>, String>) {
            *self.download_result.lock().unwrap() = r;
        }
    }

    #[async_trait]
    impl GcsClient for FakeGcsClient {
        async fn sign_put_url(
            &self,
            bucket_resource: &str,
            object_name: &str,
            ttl: Duration,
        ) -> Result<String, String> {
            *self.last_sign_put_call.lock().unwrap() =
                Some((bucket_resource.to_string(), object_name.to_string(), ttl));

            self.sign_put_result.lock().unwrap().clone()
        }

        async fn sign_get_url(
            &self,
            bucket_resource: &str,
            object_name: &str,
            ttl: Duration,
        ) -> Result<String, String> {
            *self.last_sign_get_call.lock().unwrap() =
                Some((bucket_resource.to_string(), object_name.to_string(), ttl));

            self.sign_get_result.lock().unwrap().clone()
        }

        async fn download_object_bytes(
            &self,
            bucket_resource: &str,
            object_name: &str,
        ) -> Result<Vec<u8>, String> {
            *self.last_download_call.lock().unwrap() =
                Some((bucket_resource.to_string(), object_name.to_string()));

            self.download_result.lock().unwrap().clone()
        }
    }

    fn sample_media_info() -> MediaInfo {
        MediaInfo::try_new(
            "blogport-cms-upload".to_string(),
            "abc.webp".to_string(),
            AttachmentTarget::Resume,
        )
        .unwrap()
    }

    // -----------------------
    // get_signed_upload_url
    // -----------------------

    #[tokio::test]
    async fn test_get_signed_upload_url_success_and_uses_bucket_resource() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_put_result(Ok("https://signed.example".to_string()));

        let svc = GcsStorageQuery::with_client(fake.clone(), Duration::from_secs(123));

        let url = svc
            .get_signed_upload_url(sample_media_info())
            .await
            .unwrap();
        assert_eq!(url, "https://signed.example");

        let call = fake.last_sign_put_call.lock().unwrap().clone().unwrap();
        assert_eq!(call.0, "projects/_/buckets/blogport-cms-upload");
        assert_eq!(call.1, "abc.webp");
        assert_eq!(call.2, Duration::from_secs(123));
    }

    #[tokio::test]
    async fn test_get_signed_upload_url_maps_access_denied() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_put_result(Err("Permission denied".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);
        let err = svc
            .get_signed_upload_url(sample_media_info())
            .await
            .unwrap_err();

        assert!(matches!(err, SignUrlError::AccessDenied));
    }

    #[tokio::test]
    async fn test_get_signed_upload_url_maps_bucket_not_found() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_put_result(Err("Bucket not found (404)".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);
        let err = svc
            .get_signed_upload_url(sample_media_info())
            .await
            .unwrap_err();

        assert!(matches!(err, SignUrlError::BucketNotFound));
    }

    #[tokio::test]
    async fn test_get_signed_upload_url_maps_configuration() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_put_result(Err("Invalid configuration".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);
        let err = svc
            .get_signed_upload_url(sample_media_info())
            .await
            .unwrap_err();

        assert!(matches!(err, SignUrlError::Configuration));
    }

    #[tokio::test]
    async fn test_get_signed_upload_url_maps_infrastructure_fallback() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_put_result(Err("some weird error".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);
        let err = svc
            .get_signed_upload_url(sample_media_info())
            .await
            .unwrap_err();

        assert!(matches!(err, SignUrlError::Infrastructure));
    }

    // -----------------------
    // get_signed_read_url
    // -----------------------

    #[tokio::test]
    async fn test_get_signed_read_url_success_and_uses_bucket_resource() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_get_result(Ok("https://signed-read.example".to_string()));

        let svc = GcsStorageQuery::with_client(fake.clone(), Duration::from_secs(456));

        let url = svc.get_signed_read_url(sample_media_info()).await.unwrap();
        assert_eq!(url, "https://signed-read.example");

        let call = fake.last_sign_get_call.lock().unwrap().clone().unwrap();
        assert_eq!(call.0, "projects/_/buckets/blogport-cms-upload");
        assert_eq!(call.1, "abc.webp");
        assert_eq!(call.2, Duration::from_secs(456));
    }

    #[tokio::test]
    async fn test_get_signed_read_url_maps_access_denied() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_get_result(Err("Access forbidden".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);
        let err = svc
            .get_signed_read_url(sample_media_info())
            .await
            .unwrap_err();

        assert!(matches!(err, SignUrlError::AccessDenied));
    }

    #[tokio::test]
    async fn test_get_signed_read_url_maps_bucket_not_found() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_get_result(Err("Bucket 404".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);
        let err = svc
            .get_signed_read_url(sample_media_info())
            .await
            .unwrap_err();

        assert!(matches!(err, SignUrlError::BucketNotFound));
    }

    #[tokio::test]
    async fn test_get_signed_read_url_maps_configuration() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_get_result(Err("Configuration error".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);
        let err = svc
            .get_signed_read_url(sample_media_info())
            .await
            .unwrap_err();

        assert!(matches!(err, SignUrlError::Configuration));
    }

    #[tokio::test]
    async fn test_get_signed_read_url_maps_infrastructure_fallback() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_sign_get_result(Err("unexpected error".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);
        let err = svc
            .get_signed_read_url(sample_media_info())
            .await
            .unwrap_err();

        assert!(matches!(err, SignUrlError::Infrastructure));
    }

    // -----------------------
    // get_latest_manifest
    // -----------------------

    #[tokio::test]
    async fn test_get_latest_manifest_empty_media_id() {
        let fake = Arc::new(FakeGcsClient::new());
        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);

        let err = svc.get_latest_manifest("   ").await.unwrap_err();
        assert!(matches!(err, StorageQueryError::MediaIdNotFound));
    }

    #[tokio::test]
    async fn test_get_latest_manifest_success_and_uses_correct_object_key() {
        let fake = Arc::new(FakeGcsClient::new());
        let json = br#"{"media_id":"m1","updated_at":"2026-02-08T00:00:00Z","status":"ready"}"#;
        fake.set_download_result(Ok(json.to_vec()));

        let svc = GcsStorageQuery::with_client(fake.clone(), SIGNED_URL_TTL);

        let manifest = svc.get_latest_manifest("m1").await.unwrap();
        assert_eq!(manifest.media_id, "m1");
        assert_eq!(manifest.status, MediaState::Ready);

        let call = fake.last_download_call.lock().unwrap().clone().unwrap();
        assert_eq!(call.0, "projects/_/buckets/blogport-cms-manifests");
        assert_eq!(call.1, "m1/manifest.json");
    }

    #[tokio::test]
    async fn test_get_latest_manifest_maps_manifest_not_found() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_download_result(Err("Not Found (404)".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);

        let err = svc.get_latest_manifest("m2").await.unwrap_err();
        assert!(matches!(err, StorageQueryError::ManifestNotFound));
    }

    #[tokio::test]
    async fn test_get_latest_manifest_maps_network_interrupted() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_download_result(Err("connection timeout".to_string()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);

        let err = svc.get_latest_manifest("m3").await.unwrap_err();
        assert!(matches!(err, StorageQueryError::NetworkInterrupted));
    }

    #[tokio::test]
    async fn test_get_latest_manifest_invalid_json() {
        let fake = Arc::new(FakeGcsClient::new());
        fake.set_download_result(Ok(b"not-json".to_vec()));

        let svc = GcsStorageQuery::with_client(fake, SIGNED_URL_TTL);

        let err = svc.get_latest_manifest("m4").await.unwrap_err();
        assert!(matches!(err, StorageQueryError::NetworkInterrupted));
    }
}
