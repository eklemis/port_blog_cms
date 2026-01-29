use actix_web::{post, web, Responder};
use serde::Deserialize;

use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    shared::api::ApiResponse,
    topic::application::ports::incoming::use_cases::{
        CreateTopicCommand, CreateTopicCommandError, CreateTopicError,
    },
    AppState,
};

//
// ──────────────────────────────────────────────────────────
// Request DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize)]
struct CreateTopicRequest {
    pub title: String,
    pub description: Option<String>,
}

//
// ──────────────────────────────────────────────────────────
// Route
// ──────────────────────────────────────────────────────────
//

#[post("/api/topics")]
pub async fn create_topic_handler(
    user: VerifiedUser,
    data: web::Data<AppState>,
    payload: web::Json<CreateTopicRequest>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);

    // 1️⃣ Build command (validation happens here)
    let command =
        match CreateTopicCommand::new(owner, payload.title.clone(), payload.description.clone()) {
            Ok(cmd) => cmd,
            Err(err) => return map_command_error(err),
        };

    // 2️⃣ Execute use case
    match data.create_topic_use_case.execute(command).await {
        Ok(topic) => ApiResponse::created(topic),
        Err(err) => map_create_topic_error(err),
    }
}

//
// ──────────────────────────────────────────────────────────
// Error Mapping
// ──────────────────────────────────────────────────────────
//

fn map_command_error(err: CreateTopicCommandError) -> actix_web::HttpResponse {
    match err {
        CreateTopicCommandError::EmptyTitle => {
            ApiResponse::bad_request("EMPTY_TITLE", "Title cannot be empty")
        }
        CreateTopicCommandError::TitleTooLong => {
            ApiResponse::bad_request("TITLE_TOO_LONG", "Title must not exceed 100 characters")
        }
    }
}

fn map_create_topic_error(err: CreateTopicError) -> actix_web::HttpResponse {
    match err {
        CreateTopicError::TopicAlreadyExists => {
            ApiResponse::conflict("TOPIC_ALREADY_EXISTS", "Topic already exists")
        }
        CreateTopicError::RepositoryError(_) => ApiResponse::internal_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        auth::application::domain::entities::UserId,
        auth::application::ports::outgoing::token_provider::{
            TokenClaims, TokenError, TokenProvider,
        },
        tests::support::app_state_builder::TestAppStateBuilder,
        topic::application::ports::incoming::use_cases::{
            CreateTopicCommand, CreateTopicError, CreateTopicUseCase,
        },
        topic::application::ports::outgoing::TopicResult,
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
            unimplemented!("Not used in create_topic tests")
        }

        fn generate_refresh_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!("Not used in create_topic tests")
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
            unimplemented!("Not used in create_topic tests")
        }

        fn generate_verification_token(&self, _user_id: Uuid) -> Result<String, TokenError> {
            unimplemented!("Not used in create_topic tests")
        }

        fn verify_verification_token(&self, _token: &str) -> Result<Uuid, TokenError> {
            unimplemented!("Not used in create_topic tests")
        }
    }

    // ============================================================
    // CreateTopic Use Case Mock
    // ============================================================

    #[derive(Clone)]
    struct MockCreateTopicUseCase {
        result: Result<TopicResult, CreateTopicError>,
    }

    impl MockCreateTopicUseCase {
        fn success(topic: TopicResult) -> Self {
            Self { result: Ok(topic) }
        }

        fn already_exists() -> Self {
            Self {
                result: Err(CreateTopicError::TopicAlreadyExists),
            }
        }

        fn repo_error(msg: &str) -> Self {
            Self {
                result: Err(CreateTopicError::RepositoryError(msg.to_string())),
            }
        }
    }

    #[async_trait]
    impl CreateTopicUseCase for MockCreateTopicUseCase {
        async fn execute(
            &self,
            _command: CreateTopicCommand,
        ) -> Result<TopicResult, CreateTopicError> {
            self.result.clone()
        }
    }

    // ============================================================
    // Helpers
    // ============================================================

    async fn read_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
        let body = test::read_body(resp).await;
        serde_json::from_slice(&body).unwrap()
    }

    fn sample_topic(owner: UserId, title: &str, description: &str) -> TopicResult {
        TopicResult {
            id: Uuid::new_v4(),
            owner,
            title: title.to_string(),
            description: description.to_string(),
        }
    }

    fn bearer() -> (&'static str, &'static str) {
        ("Authorization", "Bearer test-token")
    }

    // ============================================================
    // Tests
    // ============================================================

    #[actix_web::test]
    async fn create_topic_empty_title_returns_bad_request() {
        // Arrange
        let user_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default().build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(create_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/topics")
            .insert_header(bearer())
            .set_json(serde_json::json!({
                "title": "   ",
                "description": "desc"
            }))
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "EMPTY_TITLE");
    }

    #[actix_web::test]
    async fn create_topic_title_too_long_returns_bad_request() {
        // Arrange
        let user_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default().build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(create_topic_handler),
        )
        .await;

        let long_title = "a".repeat(101);

        let req = test::TestRequest::post()
            .uri("/api/topics")
            .insert_header(bearer())
            .set_json(serde_json::json!({
                "title": long_title,
                "description": null
            }))
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "TITLE_TOO_LONG");
    }

    #[actix_web::test]
    async fn create_topic_success_returns_created() {
        // Arrange
        let user_id = Uuid::new_v4();
        let owner = UserId::from(user_id);

        let topic = sample_topic(owner.clone(), "Rust", "desc");

        let state = TestAppStateBuilder::default()
            .with_create_topic(MockCreateTopicUseCase::success(topic.clone()))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(create_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/topics")
            .insert_header(bearer())
            .set_json(serde_json::json!({
                "title": "Rust",
                "description": "desc"
            }))
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::CREATED);

        let json = read_json(resp).await;
        assert_eq!(json["success"], true);

        // Keep assertions resilient but meaningful
        assert_eq!(json["data"]["title"], "Rust");
        assert_eq!(json["data"]["description"], "desc");
    }

    #[actix_web::test]
    async fn create_topic_already_exists_returns_conflict() {
        // Arrange
        let user_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default()
            .with_create_topic(MockCreateTopicUseCase::already_exists())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(create_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/topics")
            .insert_header(bearer())
            .set_json(serde_json::json!({
                "title": "Rust",
                "description": null
            }))
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "TOPIC_ALREADY_EXISTS");
    }

    #[actix_web::test]
    async fn create_topic_repository_error_returns_internal_error() {
        // Arrange
        let user_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default()
            .with_create_topic(MockCreateTopicUseCase::repo_error("db down"))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(StubTokenProvider {
            user_id,
            is_verified: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(create_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/topics")
            .insert_header(bearer())
            .set_json(serde_json::json!({
                "title": "Rust",
                "description": "desc"
            }))
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "INTERNAL_ERROR");
    }
}
