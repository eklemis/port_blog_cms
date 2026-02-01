use actix_web::{post, web, Responder};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    auth::application::domain::entities::UserId,
    modules::project::application::ports::incoming::use_cases::AddProjectTopicError,
    shared::api::ApiResponse, AppState,
};

#[derive(Debug, Deserialize)]
pub struct AddProjectTopicRequest {
    pub topic_id: Uuid,
}

#[post("/api/projects/{project_id}/topics")]
pub async fn add_project_topic_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<AddProjectTopicRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let project_id = path.into_inner();
    let topic_id = body.topic_id;

    match data
        .project
        .add_topic
        .execute(owner, project_id, topic_id)
        .await
    {
        Ok(_) => ApiResponse::success(serde_json::json!({ "message": "OK" })),

        Err(AddProjectTopicError::ProjectNotFound) => {
            ApiResponse::not_found("PROJECT_NOT_FOUND", "Project not found")
        }

        Err(AddProjectTopicError::TopicNotFound) => {
            ApiResponse::not_found("TOPIC_NOT_FOUND", "Topic not found")
        }

        Err(AddProjectTopicError::RepositoryError(msg)) => {
            error!("Failed to add topic to project: {}", msg);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::domain::entities::UserId;
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::modules::project::application::ports::incoming::use_cases::{
        AddProjectTopicError, AddProjectTopicUseCase,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock AddProjectTopicUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockAddProjectTopicUseCase {
        result: Result<(), AddProjectTopicError>,
    }

    impl MockAddProjectTopicUseCase {
        fn success() -> Self {
            Self { result: Ok(()) }
        }

        fn error(err: AddProjectTopicError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl AddProjectTopicUseCase for MockAddProjectTopicUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _topic_id: Uuid,
        ) -> Result<(), AddProjectTopicError> {
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

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_add_project_topic_success() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_add_project_topic(MockAddProjectTopicUseCase::success())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(add_project_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .set_json(json!({ "topic_id": topic_id }))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["error"].is_null());
        assert!(body.get("data").is_some());
    }

    #[actix_web::test]
    async fn test_add_project_topic_project_not_found() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_add_project_topic(MockAddProjectTopicUseCase::error(
                AddProjectTopicError::ProjectNotFound,
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(add_project_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .set_json(json!({ "topic_id": topic_id }))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_add_project_topic_topic_not_found() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_add_project_topic(MockAddProjectTopicUseCase::error(
                AddProjectTopicError::TopicNotFound,
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(add_project_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .set_json(json!({ "topic_id": topic_id }))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "TOPIC_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_add_project_topic_repository_error_internal_error() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_add_project_topic(MockAddProjectTopicUseCase::error(
                AddProjectTopicError::RepositoryError("db down".to_string()),
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(add_project_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .set_json(json!({ "topic_id": topic_id }))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_add_project_topic_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_add_project_topic(MockAddProjectTopicUseCase::success())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(add_project_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .set_json(json!({ "topic_id": topic_id }))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
