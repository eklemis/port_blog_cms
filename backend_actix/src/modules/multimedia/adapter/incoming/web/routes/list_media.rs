use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    multimedia::application::{
        domain::entities::AttachmentTarget,
        ports::incoming::use_cases::{ListMediaCommand, MediaItem},
    },
    shared::api::ApiResponse,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListMediaPath {
    attachment_target: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct ListMediaResponse {
    rows: Vec<MediaItem>,
}

fn parse_attachment_target(s: &str) -> Result<AttachmentTarget, HttpResponse> {
    match s {
        "user" => Ok(AttachmentTarget::User),
        "resume" => Ok(AttachmentTarget::Resume),
        "project" => Ok(AttachmentTarget::Project),
        "blog_post" => Ok(AttachmentTarget::BlogPost),
        _ => Err(ApiResponse::bad_request(
            "TARGET_NOT_FOUND",
            "Target Attachment Is Not Exist",
        )),
    }
}

#[get("/api/media/{attachment_target}")]
pub async fn list_media_handler(
    user: VerifiedUser,
    path: web::Path<ListMediaPath>,
    data: web::Data<AppState>,
) -> impl Responder {
    let attachment_target = match parse_attachment_target(&path.attachment_target) {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    let command = ListMediaCommand {
        owner: UserId::from(user.user_id),
        attachment_target,
    };
    match data.multimedia.list_media.execute(command).await {
        Ok(items) => ApiResponse::success(ListMediaResponse { rows: items }),
        Err(err) => {
            println!("Error from the server: {}", err);
            return ApiResponse::internal_error();
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
        ListMediaError, ListMediaUseCase,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    // -----------------------
    // Mock use case
    // -----------------------

    #[derive(Clone)]
    struct MockListMediaUseCase {
        result: Result<Vec<MediaItem>, ListMediaError>,
    }

    impl MockListMediaUseCase {
        fn ok(items: Vec<MediaItem>) -> Self {
            Self { result: Ok(items) }
        }

        fn err(err: ListMediaError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl ListMediaUseCase for MockListMediaUseCase {
        async fn execute(
            &self,
            _command: ListMediaCommand,
        ) -> Result<Vec<MediaItem>, ListMediaError> {
            self.result.clone()
        }
    }

    // -----------------------
    // JWT helpers (same style)
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
    // Minimal MediaItem factory
    // -----------------------
    //
    // If MediaItem is a struct with public fields, build it directly here.
    // If it only has from_media_attachment(...), then you can construct a MediaAttachment and convert.
    //
    fn sample_media_item() -> MediaItem {
        // !!! adjust this to match your MediaItem shape
        MediaItem {
            media_id: Uuid::new_v4(),
            original_filename: "test.jpg".to_string(),
            status: crate::multimedia::application::domain::entities::MediaState::Ready, // adjust if needed
            attachment_target: AttachmentTarget::Resume,
            attachment_target_id: Uuid::new_v4(),
            role: crate::multimedia::application::domain::entities::MediaRole::Profile, // adjust if needed
            position: 0,
            alt_text: "".to_string(),
            caption: "".to_string(),
        }
    }

    // -----------------------
    // Success
    // -----------------------

    #[actix_web::test]
    async fn test_list_media_success() {
        let user_id = Uuid::new_v4();

        let item = sample_media_item();

        let app_state = TestAppStateBuilder::default()
            .with_list_media(MockListMediaUseCase::ok(vec![item]))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(list_media_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/media/resume")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["rows"].is_array());
        assert_eq!(body["data"]["rows"].as_array().unwrap().len(), 1);
    }

    // -----------------------
    // Invalid attachment_target (parse_attachment_target error)
    // -----------------------

    #[actix_web::test]
    async fn test_list_media_invalid_target_returns_bad_request() {
        let user_id = Uuid::new_v4();

        // Use case won't be called because parse_attachment_target fails first.
        let app_state = TestAppStateBuilder::default()
            .with_list_media(MockListMediaUseCase::ok(vec![]))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(list_media_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/media/unknown_target")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "TARGET_NOT_FOUND");
    }

    // -----------------------
    // Use case error => internal error
    // -----------------------

    #[actix_web::test]
    async fn test_list_media_use_case_error_returns_internal_error() {
        let user_id = Uuid::new_v4();

        // Pick a real ListMediaError variant that exists in your codebase.
        // If you only have one variant, use it. Otherwise, any works.
        let app_state = TestAppStateBuilder::default()
            .with_list_media(MockListMediaUseCase::err(ListMediaError::RepositoryError(
                "db down".to_string(),
            )))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(list_media_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/media/resume")
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
    async fn test_list_media_unverified_forbidden() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_list_media(MockListMediaUseCase::ok(vec![]))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(list_media_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/media/resume")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
