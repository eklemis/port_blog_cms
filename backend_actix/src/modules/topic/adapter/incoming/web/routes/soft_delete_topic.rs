// src/modules/topic/adapter/incoming/web/routes/soft_delete_topic.rs
use actix_web::{delete, web, Responder};
use uuid::Uuid;

use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    api::schemas::ErrorResponse,
    shared::api::ApiResponse,
    topic::application::ports::incoming::use_cases::SoftDeleteTopicError,
    AppState,
};

//
// ──────────────────────────────────────────────────────────
// Route
// ──────────────────────────────────────────────────────────
//

/// Soft-delete a topic
///
/// Marks the topic deleted rather than removing the row, so it stops appearing
/// in listings while existing references remain resolvable. Deleting a topic
/// owned by another user is rejected as forbidden, not hidden as not-found.
#[utoipa::path(
    delete,
    path = "/api/topics/{topic_id}",
    tag = "topics",
    params(
        ("topic_id" = Uuid, Path, description = "Identifier of the topic to delete")
    ),
    responses(
        (status = 204, description = "Topic deleted successfully"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (
            status = 403,
            description = "Email not verified, or the topic belongs to another user",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "FORBIDDEN", "message": "You are not the owner of this topic" }
            })
        ),
        (
            status = 404,
            description = "Topic not found",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "TOPIC_NOT_FOUND", "message": "Topic not found" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/topics/{topic_id}")]
pub async fn soft_delete_topic_handler(
    user: VerifiedUser,
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let topic_id = path.into_inner();

    match data
        .soft_delete_topic_use_case
        .execute(owner, topic_id)
        .await
    {
        Ok(_) => ApiResponse::no_content(),
        Err(err) => map_soft_delete_topic_error(err),
    }
}

//
// ──────────────────────────────────────────────────────────
// Error Mapping
// ──────────────────────────────────────────────────────────
//

fn map_soft_delete_topic_error(err: SoftDeleteTopicError) -> actix_web::HttpResponse {
    match err {
        SoftDeleteTopicError::TopicNotFound => {
            ApiResponse::not_found("TOPIC_NOT_FOUND", "Topic not found")
        }
        SoftDeleteTopicError::Forbidden => {
            ApiResponse::forbidden("FORBIDDEN", "You are not the owner of this topic")
        }
        SoftDeleteTopicError::DatabaseError(_) => ApiResponse::internal_error(),
    }
}

//
// ──────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;

    use crate::{
        auth::application::ports::outgoing::token_provider::{
            TokenClaims, TokenError, TokenProvider,
        },
        tests::support::app_state_builder::TestAppStateBuilder,
        topic::application::ports::incoming::use_cases::{
            SoftDeleteTopicError, SoftDeleteTopicUseCase,
        },
    };

    // ============================================================
    // TokenProvider Stub (FULL, trait-accurate)
    // ============================================================

    #[derive(Clone)]
    struct StubTokenProvider {
        user_id: Uuid,
        is_verified: bool,
    }

    impl TokenProvider for StubTokenProvider {
        fn generate_access_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!("Not used in soft_delete_topic tests")
        }

        fn generate_refresh_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!("Not used in soft_delete_topic tests")
        }

        fn verify_token(&self, _token: &str) -> Result<TokenClaims, TokenError> {
            Ok(TokenClaims {
                sub: self.user_id,
                exp: 9_999_999_999,
                iat: 0,
                nbf: 0,
                token_type: "access".to_string(),
                is_verified: self.is_verified,
            })
        }

        fn refresh_access_token(&self, _refresh_token: &str) -> Result<String, TokenError> {
            unimplemented!("Not used in soft_delete_topic tests")
        }

        fn generate_verification_token(&self, _user_id: Uuid) -> Result<String, TokenError> {
            unimplemented!("Not used in soft_delete_topic tests")
        }

        fn verify_verification_token(&self, _token: &str) -> Result<Uuid, TokenError> {
            unimplemented!("Not used in soft_delete_topic tests")
        }
    }

    // ============================================================
    // UseCase Mock
    // ============================================================

    #[derive(Clone)]
    struct MockSoftDeleteTopicUseCase {
        result: Result<(), SoftDeleteTopicError>,
    }

    impl MockSoftDeleteTopicUseCase {
        fn success() -> Self {
            Self { result: Ok(()) }
        }

        fn forbidden() -> Self {
            Self {
                result: Err(SoftDeleteTopicError::Forbidden),
            }
        }

        fn not_found() -> Self {
            Self {
                result: Err(SoftDeleteTopicError::TopicNotFound),
            }
        }

        fn db_error(msg: &str) -> Self {
            Self {
                result: Err(SoftDeleteTopicError::DatabaseError(msg.to_string())),
            }
        }
    }

    #[async_trait]
    impl SoftDeleteTopicUseCase for MockSoftDeleteTopicUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _topic_id: Uuid,
        ) -> Result<(), SoftDeleteTopicError> {
            self.result.clone()
        }
    }

    // ============================================================
    // Helpers
    // ============================================================

    fn bearer() -> (&'static str, &'static str) {
        ("Authorization", "Bearer test-token")
    }

    async fn read_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
        let body = test::read_body(resp).await;
        serde_json::from_slice(&body).unwrap()
    }

    // ============================================================
    // Tests
    // ============================================================

    #[actix_web::test]
    async fn soft_delete_topic_success_returns_no_content() {
        // Arrange
        let user_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default()
            .with_soft_delete_topic(MockSoftDeleteTopicUseCase::success())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(soft_delete_topic_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/topics/{}", topic_id))
            .insert_header(bearer())
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let body = test::read_body(resp).await;
        assert!(body.is_empty());
    }

    #[actix_web::test]
    async fn soft_delete_topic_forbidden_returns_403() {
        // Arrange
        let user_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default()
            .with_soft_delete_topic(MockSoftDeleteTopicUseCase::forbidden())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(soft_delete_topic_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/topics/{}", topic_id))
            .insert_header(bearer())
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "FORBIDDEN");
    }

    #[actix_web::test]
    async fn soft_delete_topic_not_found_returns_404() {
        // Arrange
        let user_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default()
            .with_soft_delete_topic(MockSoftDeleteTopicUseCase::not_found())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(soft_delete_topic_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/topics/{}", topic_id))
            .insert_header(bearer())
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "TOPIC_NOT_FOUND");
    }

    #[actix_web::test]
    async fn soft_delete_topic_db_error_returns_500() {
        // Arrange
        let user_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default()
            .with_soft_delete_topic(MockSoftDeleteTopicUseCase::db_error("db down"))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(soft_delete_topic_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/topics/{}", topic_id))
            .insert_header(bearer())
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn soft_delete_topic_email_not_verified_returns_403() {
        // Arrange
        let user_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default().build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: false,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(soft_delete_topic_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/topics/{}", topic_id))
            .insert_header(bearer())
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
