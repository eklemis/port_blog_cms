//! `GET /api/public/users/{username}`.

use actix_web::{get, web, Responder};
use tracing::error;

use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::application::use_cases::get_public_profile::{
    GetPublicProfileError, PublicProfile,
};
use crate::shared::api::{ApiResponse, ErrorCode};
use crate::AppState;

/// Read an author's public profile
///
/// Every public route is keyed on `{username}`, but nothing returned who that
/// was — so a public page had an anonymous header and could not introduce the
/// person whose work it was showing.
///
/// Public: no token required. Returns display name, bio and avatar only. No
/// email and no account state: this is the one endpoint that serves a user's
/// details to somebody else, so it carries the minimum a page needs.
///
/// A deleted account is reported as not found, matching the rest of the public
/// surface.
#[utoipa::path(
    get,
    path = "/api/public/users/{username}",
    tag = "users",
    params(("username" = String, Path, description = "The author's public handle")),
    responses(
        (
            status = 200,
            description = "The author's public profile",
            body = inline(SuccessResponse<PublicProfile>),
            example = json!({
                "success": true,
                "data": {
                    "username": "janedoe",
                    "full_name": "Jane Doe",
                    "bio": "Backend engineer, mostly Rust.",
                    "avatar": {
                        "media_id": "8f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d",
                        "alt_text": "Jane Doe",
                        "caption": "",
                        "role": "avatar",
                        "position": 0,
                        "variants": { "thumbnail": "/api/public/media/8f1b2c3d…/thumbnail" }
                    }
                }
            })
        ),
        (status = 404, description = "No such author, or the account is deleted", body = ErrorResponse),
    )
)]
#[get("/api/public/users/{username}")]
pub async fn get_public_profile_handler(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .get_public_profile_use_case
        .execute(&path.into_inner())
        .await
    {
        Ok(profile) => ApiResponse::success(profile),
        Err(GetPublicProfileError::NotFound) => {
            ApiResponse::not_found(ErrorCode::UserNotFound, "User not found")
        }
        Err(GetPublicProfileError::QueryError(e)) => {
            error!("Failed to read a public profile: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use serde_json::Value;

    struct MockGetPublicProfile(Result<PublicProfile, GetPublicProfileError>);

    #[async_trait]
    impl crate::auth::application::use_cases::get_public_profile::GetPublicProfileUseCase
        for MockGetPublicProfile
    {
        async fn execute(&self, _username: &str) -> Result<PublicProfile, GetPublicProfileError> {
            self.0.clone()
        }
    }

    async fn call(result: Result<PublicProfile, GetPublicProfileError>) -> (StatusCode, Value) {
        let app_state = TestAppStateBuilder::default()
            .with_public_profile(MockGetPublicProfile(result))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_profile_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/public/users/janedoe")
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        (status, test::read_body_json(resp).await)
    }

    /// No token is set on the request, so a 200 here is also the assertion
    /// that the route is public.
    #[actix_web::test]
    async fn it_serves_a_profile_without_a_token() {
        let (status, body) = call(Ok(PublicProfile {
            username: "janedoe".to_string(),
            full_name: "Jane Doe".to_string(),
            bio: Some("Rust, mostly.".to_string()),
            avatar: None,
        }))
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["username"], "janedoe");
        assert_eq!(body["data"]["full_name"], "Jane Doe");
        assert_eq!(body["data"]["bio"], "Rust, mostly.");
        assert!(body["data"]["avatar"].is_null());
        assert!(
            body["data"]["email"].is_null(),
            "the public profile must not carry an email: {body}"
        );
    }

    #[actix_web::test]
    async fn an_unknown_author_is_a_404() {
        let (status, body) = call(Err(GetPublicProfileError::NotFound)).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "USER_NOT_FOUND");
    }

    /// A read failure must not be reported as "no such author" — that would
    /// send a client caching 404s down the wrong path during an outage.
    #[actix_web::test]
    async fn a_query_failure_is_a_500() {
        let (status, _) = call(Err(GetPublicProfileError::QueryError(
            "connection refused".to_string(),
        )))
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
