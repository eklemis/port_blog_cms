use actix_web::{delete, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::ErrorResponse,
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    blog::application::ports::incoming::use_cases::ArchiveBlogPostError,
    shared::api::ApiResponse,
    AppState,
};

/// Archive a post
///
/// Soft delete: the post drops out of every listing but the row survives and can be brought back with `POST /api/blog/{post_id}/restore`. Use `DELETE /api/blog/{post_id}/hard` to remove it outright.
#[utoipa::path(
    delete,
    path = "/api/blog/{post_id}",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    responses(
        (status = 204, description = "Post archived"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (
            status = 404,
            description = "Post not found, already archived, or owned by another author",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "POST_NOT_FOUND", "message": "Blog post not found" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/blog/{post_id}")]
pub async fn archive_blog_post_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog
        .archive
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(ArchiveBlogPostError::NotFound) => {
            ApiResponse::not_found("POST_NOT_FOUND", "Blog post not found")
        }
        Err(ArchiveBlogPostError::RepositoryError(e)) => {
            error!("Repository error on blog post lifecycle: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::blog::application::ports::incoming::use_cases::ArchiveBlogPostUseCase;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;

    struct Mock {
        result: Result<(), ArchiveBlogPostError>,
    }

    #[async_trait]
    impl ArchiveBlogPostUseCase for Mock {
        async fn execute(&self, _o: UserId, _p: Uuid) -> Result<(), ArchiveBlogPostError> {
            self.result.clone()
        }
    }

    async fn call(
        result: Result<(), ArchiveBlogPostError>,
        verified: bool,
    ) -> actix_web::dev::ServiceResponse {
        let j = JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        });
        let token = j.generate_access_token(Uuid::new_v4(), verified).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_archive(Mock { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(archive_blog_post_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/blog/{}", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn succeeds_with_no_content() {
        assert_eq!(call(Ok(()), true).await.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn a_missing_post_is_not_found() {
        let resp = call(Err(ArchiveBlogPostError::NotFound), true).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "POST_NOT_FOUND");
    }

    #[actix_web::test]
    async fn a_repository_failure_is_an_internal_error() {
        let resp = call(
            Err(ArchiveBlogPostError::RepositoryError("db down".into())),
            true,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn unverified_user_is_forbidden() {
        let resp = call(Ok(()), false).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
