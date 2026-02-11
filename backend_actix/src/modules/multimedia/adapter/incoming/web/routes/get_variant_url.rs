use actix_web::{get, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::multimedia::application::domain::entities::MediaSize;
use crate::multimedia::application::ports::incoming::use_cases::{GetReadUrlError, GetUrlCommand};
use crate::shared::api::ApiResponse;
use crate::AppState;

//
// ──────────────────────────────────────────────────────────
// Path DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize)]
pub struct GetVariantPath {
    pub media_id: Uuid,
    pub media_size: MediaSize,
}

//
// ──────────────────────────────────────────────────────────
// Response DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct GetVariantUrlResponse {
    pub media_id: Uuid,
    pub size: MediaSize,
    pub url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

//
// ──────────────────────────────────────────────────────────
// Handler
// ──────────────────────────────────────────────────────────
//

#[get("/api/media/{media_id}/{media_size}")]
pub async fn get_variant_read_url_handler(
    user: VerifiedUser,
    path: web::Path<(Uuid, String)>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (media_id, media_size_raw) = path.into_inner();

    let media_size = match media_size_raw.as_str() {
        "thumbnail" => MediaSize::Thumbnail,
        "small" => MediaSize::Small,
        "medium" => MediaSize::Medium,
        "large" => MediaSize::Large,
        _ => return ApiResponse::not_found("VARIANT_NOT_FOUND", "Invalid media size"),
    };

    let command = GetUrlCommand {
        owner: user.user_id.into(),
        media_id,
        size: media_size,
    };

    match data.multimedia.create_signed_get_url.execute(command).await {
        Ok(result) => ApiResponse::success(GetVariantUrlResponse {
            media_id: result.media_id,
            size: result.size,
            url: result.url,
            expires_at: result.expires_at,
        }),

        Err(e) => map_get_read_url_error(e),
    }
}

fn map_get_read_url_error(e: GetReadUrlError) -> actix_web::HttpResponse {
    match e {
        GetReadUrlError::MediaNotFound => {
            ApiResponse::not_found("MEDIA_NOT_FOUND", "Media not found")
        }
        GetReadUrlError::VariantNotFound(size) => ApiResponse::not_found(
            "VARIANT_NOT_FOUND",
            &format!("Variant '{:?}' not found for this media", size),
        ),

        // state-related errors: 409 Conflict makes sense
        GetReadUrlError::MediaProcessing => ApiResponse::error(
            actix_web::http::StatusCode::CONFLICT,
            "MEDIA_PROCESSING",
            "Media is still being processed",
        ),
        GetReadUrlError::MediaPending => ApiResponse::error(
            actix_web::http::StatusCode::CONFLICT,
            "MEDIA_PENDING",
            "Media is pending upload",
        ),
        GetReadUrlError::MediaFailed => ApiResponse::error(
            actix_web::http::StatusCode::CONFLICT,
            "MEDIA_FAILED",
            "Media processing failed",
        ),

        // infra errors
        GetReadUrlError::StorageError(msg) => {
            error!("Storage error creating read URL: {}", msg);
            ApiResponse::error(
                actix_web::http::StatusCode::BAD_GATEWAY,
                "STORAGE_ERROR",
                "Failed to generate read URL",
            )
        }
        GetReadUrlError::QueryError(msg) => {
            error!("Query error creating read URL: {}", msg);
            ApiResponse::internal_error()
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
    use crate::multimedia::application::ports::incoming::use_cases::{
        GetUrlResult, GetVariantReadUrlUseCase,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    // -----------------------
    // Mock use case
    // -----------------------

    #[derive(Clone)]
    struct MockCreateSignedGetUrlUseCase {
        result: Result<GetUrlResult, GetReadUrlError>,
    }

    impl MockCreateSignedGetUrlUseCase {
        fn ok(media_id: Uuid, size: MediaSize, url: &str) -> Self {
            Self {
                result: Ok(GetUrlResult {
                    media_id,
                    size,
                    url: url.to_string(),
                    expires_at: chrono::Utc::now(),
                }),
            }
        }

        fn err(err: GetReadUrlError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl GetVariantReadUrlUseCase for MockCreateSignedGetUrlUseCase {
        async fn execute(&self, _command: GetUrlCommand) -> Result<GetUrlResult, GetReadUrlError> {
            self.result.clone()
        }
    }

    // -----------------------
    // JWT helpers (same as init_upload tests)
    // -----------------------

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

    // -----------------------
    // Success
    // -----------------------

    #[actix_web::test]
    async fn test_get_variant_read_url_success() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::ok(
                media_id,
                MediaSize::Thumbnail,
                "https://example.com/read-url",
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/thumbnail", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["media_id"], media_id.to_string());
        assert_eq!(body["data"]["url"], "https://example.com/read-url");
        assert!(body["data"]["expires_at"].is_string());
    }

    // -----------------------
    // Error arms in map_get_read_url_error
    // -----------------------

    #[actix_web::test]
    async fn test_get_variant_read_url_media_not_found() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::err(
                GetReadUrlError::MediaNotFound,
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/thumbnail", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "MEDIA_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_variant_read_url_variant_not_found() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::err(
                GetReadUrlError::VariantNotFound(MediaSize::Small),
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/small", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "VARIANT_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_variant_read_url_media_processing() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::err(
                GetReadUrlError::MediaProcessing,
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/thumbnail", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "MEDIA_PROCESSING");
    }

    #[actix_web::test]
    async fn test_get_variant_read_url_media_pending() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::err(
                GetReadUrlError::MediaPending,
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/thumbnail", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "MEDIA_PENDING");
    }

    #[actix_web::test]
    async fn test_get_variant_read_url_media_failed() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::err(
                GetReadUrlError::MediaFailed,
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/thumbnail", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "MEDIA_FAILED");
    }

    #[actix_web::test]
    async fn test_get_variant_read_url_storage_error() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::err(
                GetReadUrlError::StorageError("gcs down".to_string()),
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/thumbnail", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "STORAGE_ERROR");
    }

    #[actix_web::test]
    async fn test_get_variant_read_url_query_error() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::err(
                GetReadUrlError::QueryError("db down".to_string()),
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/thumbnail", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    // -----------------------
    // Auth: unverified user forbidden
    // -----------------------

    #[actix_web::test]
    async fn test_get_variant_read_url_unverified_forbidden() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_signed_get_url(MockCreateSignedGetUrlUseCase::ok(
                media_id,
                MediaSize::Thumbnail,
                "https://example.com/read-url",
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_variant_read_url_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}/thumbnail", media_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
