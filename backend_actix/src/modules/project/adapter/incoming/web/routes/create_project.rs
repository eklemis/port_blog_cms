use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::CreateProjectError;
use crate::modules::project::application::ports::outgoing::project_repository::CreateProjectData;
use crate::shared::api::ApiResponse;
use crate::AppState;

//
// ──────────────────────────────────────────────────────────
// Request DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProjectRequest {
    pub title: String,
    pub slug: String,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub screenshots: Vec<String>,
    pub repo_url: Option<String>,
    pub live_demo_url: Option<String>,
}

//
// ──────────────────────────────────────────────────────────
// Handler
// ──────────────────────────────────────────────────────────
//

#[post("/api/projects")]
pub async fn create_project_handler(
    user: VerifiedUser,
    req: web::Json<CreateProjectRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let req = req.into_inner();

    let project_data = CreateProjectData {
        owner: UserId::from(user.user_id),
        title: req.title,
        slug: req.slug,
        description: req.description,
        tech_stack: req.tech_stack,
        screenshots: req.screenshots,
        repo_url: req.repo_url,
        live_demo_url: req.live_demo_url,
    };

    match data.project.create.execute(project_data).await {
        Ok(created) => ApiResponse::created(created),

        Err(CreateProjectError::SlugAlreadyExists) => {
            ApiResponse::conflict("SLUG_ALREADY_EXISTS", "Project slug already exists")
        }

        Err(CreateProjectError::RepositoryError(e)) => {
            error!("Repository error creating project: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::Value;

    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::domain::entities::UserId;
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;

    use crate::modules::project::application::ports::incoming::use_cases::{
        CreateProjectError, CreateProjectUseCase,
    };
    use crate::modules::project::application::ports::outgoing::project_repository::{
        CreateProjectData, ProjectResult,
    };

    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock Create Project Use Case
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockCreateProjectUseCase {
        result: Result<ProjectResult, CreateProjectError>,
    }

    impl MockCreateProjectUseCase {
        fn success(data: ProjectResult) -> Self {
            Self { result: Ok(data) }
        }

        fn error(err: CreateProjectError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl CreateProjectUseCase for MockCreateProjectUseCase {
        async fn execute(
            &self,
            _data: CreateProjectData,
        ) -> Result<ProjectResult, CreateProjectError> {
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

    fn base_create_request() -> CreateProjectRequest {
        CreateProjectRequest {
            title: "My Project".to_string(),
            slug: "my-project".to_string(),
            description: "desc".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: vec!["img.png".to_string()],
            repo_url: Some("https://github.com/x/y".to_string()),
            live_demo_url: None,
        }
    }

    fn project_result(owner_uuid: Uuid) -> ProjectResult {
        ProjectResult {
            id: Uuid::new_v4(),
            owner: UserId::from(owner_uuid),
            title: "My Project".to_string(),
            slug: "my-project".to_string(),
            description: "desc".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: vec!["img.png".to_string()],
            repo_url: Some("https://github.com/x/y".to_string()),
            live_demo_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /* --------------------------------------------------
     * Success Case
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_create_project_success() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_project_use_case(MockCreateProjectUseCase::success(project_result(
                user_id,
            )))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(create_project_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/projects")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&base_create_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);

        let data = body["data"].clone();
        assert_eq!(data["title"], "My Project");
        assert_eq!(data["slug"], "my-project");
    }

    /* --------------------------------------------------
     * Error Cases
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_create_project_slug_exists_conflict() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_project_use_case(MockCreateProjectUseCase::error(
                CreateProjectError::SlugAlreadyExists,
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(create_project_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/projects")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&base_create_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "SLUG_ALREADY_EXISTS");
    }

    #[actix_web::test]
    async fn test_create_project_repository_error_internal_error() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_project_use_case(MockCreateProjectUseCase::error(
                CreateProjectError::RepositoryError("db down".to_string()),
            ))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(create_project_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/projects")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&base_create_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    /* --------------------------------------------------
     * Auth Case (same behavior as CV)
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_create_project_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_project_use_case(MockCreateProjectUseCase::success(project_result(
                user_id,
            )))
            .build();

        let jwt = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(create_project_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/projects")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .set_json(&base_create_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
