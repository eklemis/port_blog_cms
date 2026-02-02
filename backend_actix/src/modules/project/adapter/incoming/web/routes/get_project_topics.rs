use actix_web::{get, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    auth::application::domain::entities::UserId,
    modules::project::application::ports::incoming::use_cases::GetProjectTopicsError,
    shared::api::ApiResponse, AppState,
};

#[get("/api/projects/{project_id}/topics")]
pub async fn get_project_topics_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let project_id = path.into_inner();

    match data.project.get_topics.execute(owner, project_id).await {
        Ok(topics) => ApiResponse::success(topics),

        Err(GetProjectTopicsError::ProjectNotFound) => {
            ApiResponse::not_found("PROJECT_NOT_FOUND", "Project not found")
        }

        Err(GetProjectTopicsError::QueryFailed(msg)) => {
            error!("Failed to get project topics: {}", msg);
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
    use crate::modules::project::application::ports::incoming::use_cases::{
        GetProjectTopicsError, GetProjectTopicsUseCase,
    };
    use crate::project::application::ports::outgoing::project_query::ProjectTopicItem;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock GetProjectTopicsUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockGetProjectTopicsUseCase {
        result: Result<Vec<ProjectTopicItem>, GetProjectTopicsError>,
    }

    impl MockGetProjectTopicsUseCase {
        fn success(items: Vec<ProjectTopicItem>) -> Self {
            Self { result: Ok(items) }
        }

        fn error(err: GetProjectTopicsError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl GetProjectTopicsUseCase for MockGetProjectTopicsUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<Vec<ProjectTopicItem>, GetProjectTopicsError> {
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
    async fn test_get_project_topics_success() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let topics = vec![
            ProjectTopicItem {
                id: Uuid::new_v4(),
                title: "Rust".to_string(),
                description: "Systems".to_string(),
            },
            ProjectTopicItem {
                id: Uuid::new_v4(),
                title: "Actix".to_string(),
                description: "Web".to_string(),
            },
        ];

        let app_state = TestAppStateBuilder::default()
            .with_get_project_topics(MockGetProjectTopicsUseCase::success(topics.clone()))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_project_topics_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body.get("data").is_some());

        // spot-check data is an array and first item has fields
        assert!(body["data"].is_array());
        assert_eq!(body["data"].as_array().unwrap().len(), 2);
        assert!(body["data"][0].get("id").is_some());
        assert_eq!(body["data"][0]["title"], "Rust");
    }

    #[actix_web::test]
    async fn test_get_project_topics_project_not_found() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_get_project_topics(MockGetProjectTopicsUseCase::error(
                GetProjectTopicsError::ProjectNotFound,
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_project_topics_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_project_topics_query_failed_internal_error() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_get_project_topics(MockGetProjectTopicsUseCase::error(
                GetProjectTopicsError::QueryFailed("db down".to_string()),
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_project_topics_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_get_project_topics_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_get_project_topics(MockGetProjectTopicsUseCase::success(vec![]))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_project_topics_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/projects/{}/topics", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
