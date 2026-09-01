use crate::shared::api::ErrorCode;
use actix_web::{delete, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::ErrorResponse,
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    multimedia::application::ports::incoming::use_cases::DeleteMediaError,
    shared::api::ApiResponse,
    AppState,
};

/// Delete a media item
///
/// Soft delete: `deleted_at` is stamped and the item drops out of listings and
/// signed-URL requests immediately, since every read path filters on it. The
/// stored objects are left alone — the upload bucket is reaped by a GCS
/// lifecycle rule, and derived variants live in a separate bucket.
///
/// Media belonging to another user reports 404 rather than 403, so the endpoint
/// cannot be used to probe for media ids.
#[utoipa::path(
    delete,
    path = "/api/media/{media_id}",
    tag = "media",
    params(
        ("media_id" = Uuid, Path, description = "Identifier of the media to delete")
    ),
    responses(
        (status = 204, description = "Media deleted, or was already deleted"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (
            status = 404,
            description = "Media not found, or owned by another user",
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
#[delete("/api/media/{media_id}")]
pub async fn delete_media_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let media_id = path.into_inner();

    match data
        .multimedia
        .delete_media
        .execute(UserId::from(user.user_id), media_id)
        .await
    {
        Ok(()) => ApiResponse::no_content(),

        Err(DeleteMediaError::MediaNotFound) => {
            ApiResponse::not_found(ErrorCode::MediaNotFound, "Media not found")
        }

        Err(DeleteMediaError::RepositoryError(e)) => {
            error!("Repository error deleting media: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::multimedia::application::ports::incoming::use_cases::DeleteMediaUseCase;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;

    struct MockDeleteMedia {
        result: Result<(), DeleteMediaError>,
    }

    #[async_trait]
    impl DeleteMediaUseCase for MockDeleteMedia {
        async fn execute(&self, _o: UserId, _m: Uuid) -> Result<(), DeleteMediaError> {
            self.result.clone()
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
        result: Result<(), DeleteMediaError>,
        verified: bool,
    ) -> actix_web::dev::ServiceResponse {
        let jwt = jwt_service();
        let token = jwt.generate_access_token(Uuid::new_v4(), verified).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_delete_media(MockDeleteMedia { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(delete_media_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/media/{}", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn deletes_and_returns_no_content() {
        assert_eq!(call(Ok(()), true).await.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn missing_media_is_not_found() {
        let resp = call(Err(DeleteMediaError::MediaNotFound), true).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "MEDIA_NOT_FOUND");
    }

    #[actix_web::test]
    async fn repository_error_is_internal_error() {
        let resp = call(
            Err(DeleteMediaError::RepositoryError("db down".into())),
            true,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn unverified_user_is_forbidden() {
        let resp = call(Ok(()), false).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
