// src/modules/multimedia/adapter/incoming/web/routes/init_upload.rs

use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::multimedia::application::domain::entities::{AttachmentTarget, MediaRole};
use crate::multimedia::application::ports::incoming::use_cases::{
    CreateAttachmentCommand, CreateMediaCommand, CreateUrlError, UploadUrlCommandError,
};
use crate::shared::api::ApiResponse;
use crate::AppState;

//
// ──────────────────────────────────────────────────────────
// Request DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitUploadRequest {
    // File metadata
    pub file_name: String,
    pub mime_type: String,
    pub file_size_bytes: u64,

    #[serde(default)]
    pub width_px: Option<u32>,

    #[serde(default)]
    pub height_px: Option<u32>,

    // Attachment metadata
    pub attachment_target: AttachmentTarget,
    pub attachment_target_id: Uuid,
    pub role: MediaRole,

    #[serde(default)]
    pub position: u8,

    #[serde(default)]
    pub alt_text: Option<String>,

    #[serde(default)]
    pub caption: Option<String>,
}

//
// ──────────────────────────────────────────────────────────
// Response DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitUploadResponse {
    pub upload_url: String,
}

//
// ──────────────────────────────────────────────────────────
// Handler
// ──────────────────────────────────────────────────────────
//

#[post("/api/media/upload-url")]
pub async fn init_upload_handler(
    user: VerifiedUser,
    req: web::Json<InitUploadRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let req = req.into_inner();
    let policy = &data.multimedia_upload_policy;

    // Build media command
    let media_command = match CreateMediaCommand::builder()
        .owner(user.user_id.into())
        .file_name(req.file_name)
        .mime_type(req.mime_type)
        .file_size_bytes(req.file_size_bytes)
        .width_px(req.width_px)
        .height_px(req.height_px)
        .build(policy)
    {
        Ok(cmd) => cmd,
        Err(e) => return map_command_error(e),
    };

    // Build attachment command
    let attachment_command = match CreateAttachmentCommand::builder()
        .owner(user.user_id.into())
        .attachment_target(req.attachment_target)
        .attachment_target_id(req.attachment_target_id)
        .role(req.role)
        .position(req.position)
        .alt_text(req.alt_text.unwrap_or_default())
        .caption(req.caption.unwrap_or_default())
        .build()
    {
        Ok(cmd) => cmd,
        Err(e) => return map_command_error(e),
    };

    // Execute use case
    match data
        .multimedia
        .create_signed_post_url
        .execute(media_command, attachment_command)
        .await
    {
        Ok(upload_url) => ApiResponse::created(InitUploadResponse { upload_url }),

        Err(CreateUrlError::StorageError(e)) => {
            error!("Storage error creating upload URL: {}", e);
            error!("Full error details: {:?}", e);
            ApiResponse::error(
                actix_web::http::StatusCode::BAD_GATEWAY,
                "STORAGE_ERROR",
                "Failed to generate upload URL",
            )
        }

        Err(CreateUrlError::RepositoryError(e)) => {
            error!("Repository error creating upload URL: {}", e);
            ApiResponse::internal_error()
        }
    }
}

fn map_command_error(e: UploadUrlCommandError) -> actix_web::HttpResponse {
    match e {
        UploadUrlCommandError::MissingField(field) => {
            ApiResponse::bad_request("MISSING_FIELD", &format!("Missing field: {}", field))
        }
        UploadUrlCommandError::InvalidFileName => {
            ApiResponse::bad_request("INVALID_FILE_NAME", "Invalid file name")
        }
        UploadUrlCommandError::FileTooLarge {
            max_bytes,
            actual_bytes,
        } => ApiResponse::bad_request(
            "FILE_TOO_LARGE",
            &format!(
                "File too large (max {} bytes, got {} bytes)",
                max_bytes, actual_bytes
            ),
        ),
        UploadUrlCommandError::InvalidDimensions {
            max_px,
            width_px,
            height_px,
        } => ApiResponse::bad_request(
            "INVALID_DIMENSIONS",
            &format!(
                "Invalid dimensions (max {}px, got {}x{})",
                max_px, width_px, height_px
            ),
        ),
        UploadUrlCommandError::InvalidMimeType(mime) => {
            ApiResponse::bad_request("INVALID_MIME_TYPE", &format!("Invalid mime type: {}", mime))
        }
        UploadUrlCommandError::InvalidExtension(ext) => ApiResponse::bad_request(
            "INVALID_EXTENSION",
            &format!("Invalid file extension: {}", ext),
        ),
        UploadUrlCommandError::MimeExtensionMismatch { mime_type, ext } => {
            ApiResponse::bad_request(
                "MIME_EXTENSION_MISMATCH",
                &format!("Mime type {} does not match extension {}", mime_type, ext),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::multimedia::application::ports::incoming::use_cases::CreateUploadMediaUrlUseCase;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock Create Upload URL Use Case
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockCreateUploadUrlUseCase {
        result: Result<String, CreateUrlError>,
    }

    impl MockCreateUploadUrlUseCase {
        fn success(url: String) -> Self {
            Self { result: Ok(url) }
        }

        fn storage_error(msg: &str) -> Self {
            Self {
                result: Err(CreateUrlError::StorageError(msg.to_string())),
            }
        }

        fn repo_error(msg: &str) -> Self {
            Self {
                result: Err(CreateUrlError::RepositoryError(msg.to_string())),
            }
        }
    }

    #[async_trait]
    impl CreateUploadMediaUrlUseCase for MockCreateUploadUrlUseCase {
        async fn execute(
            &self,
            _media_command: CreateMediaCommand,
            _attachment_command: CreateAttachmentCommand,
        ) -> Result<String, CreateUrlError> {
            self.result.clone()
        }
    }

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn jwt_service() -> JwtTokenService {
        JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        })
    }

    fn token(user_id: Uuid, verified: bool) -> String {
        jwt_service()
            .generate_access_token(user_id, verified)
            .unwrap()
    }

    fn base_upload_request() -> InitUploadRequest {
        InitUploadRequest {
            file_name: "avatar.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            file_size_bytes: 102400,
            width_px: Some(800),
            height_px: Some(600),
            attachment_target: AttachmentTarget::User,
            attachment_target_id: Uuid::new_v4(),
            role: MediaRole::Avatar,
            position: 0,
            alt_text: None,
            caption: None,
        }
    }

    /* --------------------------------------------------
     * Success Case
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_init_upload_success() {
        let user_id = Uuid::new_v4();
        let upload_url = "https://storage.googleapis.com/signed-url".to_string();

        let app_state = TestAppStateBuilder::default()
            .with_create_upload_media_url(MockCreateUploadUrlUseCase::success(upload_url.clone()))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(init_upload_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/media/upload-url")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&base_upload_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);

        let data = body["data"].clone();
        assert_eq!(data["uploadUrl"], upload_url);
    }

    /* --------------------------------------------------
     * Error Cases
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_init_upload_storage_error() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_upload_media_url(MockCreateUploadUrlUseCase::storage_error(
                "GCS unavailable",
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(init_upload_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/media/upload-url")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&base_upload_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "STORAGE_ERROR");
    }

    #[actix_web::test]
    async fn test_init_upload_repository_error() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_upload_media_url(MockCreateUploadUrlUseCase::repo_error("db down"))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(init_upload_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/media/upload-url")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&base_upload_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    /* --------------------------------------------------
     * Validation Error Cases
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_init_upload_file_too_large() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default().build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(init_upload_handler),
        )
        .await;

        let mut request = base_upload_request();
        request.file_size_bytes = 10 * 1024 * 1024; // 10MB (over 5MB limit)

        let req = test::TestRequest::post()
            .uri("/api/media/upload-url")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "FILE_TOO_LARGE");
    }

    #[actix_web::test]
    async fn test_init_upload_invalid_mime_type() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default().build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(init_upload_handler),
        )
        .await;

        let mut request = base_upload_request();
        request.mime_type = "application/pdf".to_string();

        let req = test::TestRequest::post()
            .uri("/api/media/upload-url")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_MIME_TYPE");
    }

    #[actix_web::test]
    async fn test_init_upload_invalid_extension() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default().build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(init_upload_handler),
        )
        .await;

        let mut request = base_upload_request();
        request.file_name = "avatar.pdf".to_string();

        let req = test::TestRequest::post()
            .uri("/api/media/upload-url")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_EXTENSION");
    }

    /* --------------------------------------------------
     * Auth Case
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_init_upload_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_upload_media_url(MockCreateUploadUrlUseCase::success(
                "https://example.com".to_string(),
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(init_upload_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/media/upload-url")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .set_json(&base_upload_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
