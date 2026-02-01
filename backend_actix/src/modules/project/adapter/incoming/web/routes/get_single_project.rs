use actix_web::{get, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    auth::application::domain::entities::UserId,
    modules::project::application::ports::incoming::use_cases::GetSingleProjectError,
    shared::api::ApiResponse, AppState,
};

#[get("/api/projects/{project_id}")]
pub async fn get_project_by_id_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let project_id = path.into_inner();

    match data
        .project
        .get_single
        .execute(UserId::from(user.user_id), project_id)
        .await
    {
        Ok(project) => ApiResponse::success(project),

        Err(GetSingleProjectError::NotFound) => {
            ApiResponse::not_found("PROJECT_NOT_FOUND", "Project not found")
        }

        Err(GetSingleProjectError::RepositoryError(e)) => {
            error!("Repository error fetching project {}: {}", project_id, e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::domain::entities::UserId;
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::modules::project::application::ports::incoming::use_cases::{
        GetSingleProjectError, GetSingleProjectUseCase,
    };
    use crate::modules::project::application::ports::outgoing::project_query::ProjectView;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    /* --------------------------------------------------
     * Mock GetSingleProjectUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockGetSingleProjectUseCase {
        result: Result<ProjectView, GetSingleProjectError>,
    }

    impl MockGetSingleProjectUseCase {
        fn success(view: ProjectView) -> Self {
            Self { result: Ok(view) }
        }

        fn error(err: GetSingleProjectError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl GetSingleProjectUseCase for MockGetSingleProjectUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<ProjectView, GetSingleProjectError> {
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

    fn sample_project_view(owner: UserId, project_id: Uuid) -> ProjectView {
        ProjectView {
            id: project_id,
            owner,
            title: "Test Project".to_string(),
            slug: "test-project".to_string(),
            description: "desc".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: vec!["img.png".to_string()],
            repo_url: None,
            live_demo_url: None,
            topic_ids: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_get_project_by_id_success() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let view = sample_project_view(UserId::from(user_id), project_id);

        let app_state = TestAppStateBuilder::default()
            .with_get_single_project(MockGetSingleProjectUseCase::success(view.clone()))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_project_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["id"].as_str().unwrap(), project_id.to_string());
        assert_eq!(body["data"]["slug"], "test-project");
    }

    #[actix_web::test]
    async fn test_get_project_by_id_not_found() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_get_single_project(MockGetSingleProjectUseCase::error(
                GetSingleProjectError::NotFound,
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_project_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_project_by_id_repository_error() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_get_single_project(MockGetSingleProjectUseCase::error(
                GetSingleProjectError::RepositoryError("db down".to_string()),
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_project_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_get_project_by_id_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let view = sample_project_view(UserId::from(user_id), project_id);

        let app_state = TestAppStateBuilder::default()
            .with_get_single_project(MockGetSingleProjectUseCase::success(view))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_project_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
