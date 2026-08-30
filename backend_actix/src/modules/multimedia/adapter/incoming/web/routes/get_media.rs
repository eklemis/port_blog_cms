use actix_web::{get, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    multimedia::application::ports::incoming::use_cases::{GetMediaError, MediaDetail},
    shared::api::ApiResponse,
    AppState,
};

/// Get one media item
///
/// Returns the item along with `available_sizes`, the variant sizes that are
/// ready to read. Poll this after an upload: `status` moves to `ready` and
/// `available_sizes` fills in once the processing pipeline publishes its
/// manifest. Fetch the bytes with `GET /api/media/{media_id}/{media_size}`.
///
/// Media that is missing, soft-deleted, or owned by another user all report
/// 404, so the endpoint cannot be used to probe for media ids.
#[utoipa::path(
    get,
    path = "/api/media/{media_id}",
    tag = "media",
    params(
        ("media_id" = Uuid, Path, description = "Identifier of the media")
    ),
    responses(
        (
            status = 200,
            description = "Media retrieved successfully",
            body = inline(SuccessResponse<MediaDetail>),
            example = json!({
                "success": true,
                "data": {
                    "media_id": "123e4567-e89b-12d3-a456-426614174000",
                    "original_filename": "photo.png",
                    "status": "ready",
                    "attachment_target": "Resume",
                    "attachment_target_id": "987e6543-e21b-12d3-a456-426614174000",
                    "role": "Profile",
                    "position": 0,
                    "alt_text": "Profile photo",
                    "caption": "",
                    "available_sizes": ["thumbnail", "small", "medium", "large"]
                }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (
            status = 404,
            description = "Media not found, deleted, or owned by another user",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "MEDIA_NOT_FOUND", "message": "Media not found" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/media/{media_id}")]
pub async fn get_media_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let media_id = path.into_inner();

    match data
        .multimedia
        .get_media
        .execute(UserId::from(user.user_id), media_id)
        .await
    {
        Ok(detail) => ApiResponse::success(detail),

        Err(GetMediaError::MediaNotFound) => {
            ApiResponse::not_found("MEDIA_NOT_FOUND", "Media not found")
        }

        Err(GetMediaError::QueryError(e)) => {
            error!("Query error fetching media: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaSize, MediaState,
    };
    use crate::multimedia::application::ports::incoming::use_cases::GetMediaUseCase;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;

    struct MockGetMedia {
        result: Result<MediaDetail, GetMediaError>,
    }

    #[async_trait]
    impl GetMediaUseCase for MockGetMedia {
        async fn execute(&self, _o: UserId, _m: Uuid) -> Result<MediaDetail, GetMediaError> {
            self.result.clone()
        }
    }

    fn a_detail(sizes: Vec<MediaSize>) -> MediaDetail {
        MediaDetail {
            media_id: Uuid::new_v4(),
            original_filename: "photo.png".into(),
            status: MediaState::Ready,
            attachment_target: AttachmentTarget::Resume,
            attachment_target_id: Uuid::new_v4(),
            role: MediaRole::Profile,
            position: 0,
            alt_text: "alt".into(),
            caption: "cap".into(),
            available_sizes: sizes,
        }
    }

    fn jwt_service() -> JwtTokenService {
        JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
            password_reset_expiry: 3600,
        })
    }

    async fn call(
        result: Result<MediaDetail, GetMediaError>,
        verified: bool,
    ) -> actix_web::dev::ServiceResponse {
        let jwt = jwt_service();
        let token = jwt.generate_access_token(Uuid::new_v4(), verified).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_get_media(MockGetMedia { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(get_media_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/media/{}", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn returns_the_media_with_available_sizes() {
        let resp = call(Ok(a_detail(vec![MediaSize::Thumbnail])), true).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["status"], "ready");
        assert_eq!(body["data"]["available_sizes"][0], "thumbnail");
    }

    #[actix_web::test]
    async fn missing_media_is_not_found() {
        let resp = call(Err(GetMediaError::MediaNotFound), true).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "MEDIA_NOT_FOUND");
    }

    #[actix_web::test]
    async fn query_error_is_internal_error() {
        let resp = call(Err(GetMediaError::QueryError("db down".into())), true).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn unverified_user_is_forbidden() {
        let resp = call(Ok(a_detail(vec![])), false).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }

    /// `/api/media/{media_id}` and `/api/media/by-target/{target}` share a
    /// prefix. This is the reason the listing route was moved, so it is worth
    /// pinning that the literal segment still wins and is not swallowed as a
    /// media id.
    #[actix_web::test]
    async fn by_target_is_not_shadowed_by_the_single_media_route() {
        use crate::multimedia::adapter::incoming::web::routes::list_media_handler;

        let jwt = jwt_service();
        let token = jwt.generate_access_token(Uuid::new_v4(), true).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_get_media(MockGetMedia {
                            result: Err(GetMediaError::MediaNotFound),
                        })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(list_media_handler)
                .service(get_media_handler),
        )
        .await;

        // "not-a-target" is not a valid AttachmentTarget. If the listing route
        // handled it we get 400 TARGET_NOT_FOUND; if the single-media route
        // had shadowed it we would get 404 MEDIA_NOT_FOUND instead.
        let req = test::TestRequest::get()
            .uri("/api/media/by-target/not-a-target")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "TARGET_NOT_FOUND");
    }
}
