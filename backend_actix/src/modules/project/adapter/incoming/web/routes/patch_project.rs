use actix_web::{patch, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::PatchProjectError;
use crate::modules::project::application::ports::outgoing::project_repository::{
    PatchField, PatchProjectData,
};
use crate::shared::api::ApiResponse;
use crate::AppState;

//
// ──────────────────────────────────────────────────────────
// Request DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize, Serialize)]
pub struct PatchProjectRequest {
    #[serde(default)]
    pub title: PatchField<String>,

    #[serde(default)]
    pub description: PatchField<String>,

    #[serde(default)]
    pub tech_stack: PatchField<Vec<String>>,

    #[serde(default)]
    pub screenshots: PatchField<Vec<String>>,

    #[serde(default)]
    pub repo_url: PatchField<String>,

    #[serde(default)]
    pub live_demo_url: PatchField<String>,
}

impl From<PatchProjectRequest> for PatchProjectData {
    fn from(req: PatchProjectRequest) -> Self {
        PatchProjectData {
            title: req.title,
            description: req.description,
            tech_stack: req.tech_stack,
            screenshots: req.screenshots,
            repo_url: req.repo_url,
            live_demo_url: req.live_demo_url,
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Handler
// ──────────────────────────────────────────────────────────
//

#[patch("/api/projects/{project_id}")]
pub async fn patch_project_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    req: web::Json<PatchProjectRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let project_id = path.into_inner();
    let owner = UserId::from(user.user_id);
    let patch_data: PatchProjectData = req.into_inner().into();

    match data
        .project
        .patch
        .execute(owner, project_id, patch_data)
        .await
    {
        Ok(updated) => ApiResponse::success(updated),

        Err(PatchProjectError::NotFound) => {
            ApiResponse::not_found("PROJECT_NOT_FOUND", "Project not found")
        }

        Err(PatchProjectError::RepositoryError(e)) => {
            error!("Repository error patching project {}: {}", project_id, e);
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
    use serde_json::{json, Value};
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::domain::entities::UserId;
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;

    use crate::modules::project::application::ports::incoming::use_cases::{
        PatchProjectError, PatchProjectUseCase,
    };
    use crate::modules::project::application::ports::outgoing::project_repository::{
        PatchProjectData, ProjectResult,
    };

    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock PatchProjectUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockPatchProjectUseCase {
        result: Result<ProjectResult, PatchProjectError>,
    }

    impl MockPatchProjectUseCase {
        fn success(data: ProjectResult) -> Self {
            Self { result: Ok(data) }
        }

        fn error(err: PatchProjectError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl PatchProjectUseCase for MockPatchProjectUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _data: PatchProjectData,
        ) -> Result<ProjectResult, PatchProjectError> {
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

    fn patch_title_body() -> Value {
        // IMPORTANT: fields omitted => PatchField::Unset on the server side
        json!({
            "title": "Updated Title"
        })
    }

    fn project_result(owner_uuid: Uuid, project_id: Uuid) -> ProjectResult {
        ProjectResult {
            id: project_id,
            owner: UserId::from(owner_uuid),
            title: "Updated Title".to_string(),
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
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_patch_project_success() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_patch_project(MockPatchProjectUseCase::success(project_result(
                user_id, project_id,
            )))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_project_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&patch_title_body())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;

        assert_eq!(body["success"], true);
        assert!(body["error"].is_null());

        let data = &body["data"];
        assert_eq!(data["id"].as_str().unwrap(), project_id.to_string());
        assert_eq!(data["title"], "Updated Title");
        assert_eq!(data["slug"], "my-project");
    }

    #[actix_web::test]
    async fn test_patch_project_not_found() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_patch_project(MockPatchProjectUseCase::error(PatchProjectError::NotFound))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_project_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&patch_title_body())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
        assert_eq!(body["error"]["message"], "Project not found");
    }

    #[actix_web::test]
    async fn test_patch_project_repository_error_internal_error() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_patch_project(MockPatchProjectUseCase::error(
                PatchProjectError::RepositoryError("db down".to_string()),
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_project_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&patch_title_body())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_patch_project_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_patch_project(MockPatchProjectUseCase::success(project_result(
                user_id, project_id,
            )))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_project_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .set_json(&patch_title_body())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // Optional but consistent with your other tests: verify error shape
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
