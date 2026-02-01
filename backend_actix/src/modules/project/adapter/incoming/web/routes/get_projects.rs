use actix_web::{get, web, Responder};
use serde::Deserialize;
use tracing::error;

use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::GetProjectsError;
use crate::modules::project::application::ports::outgoing::project_query::{
    PageRequest, ProjectListFilter, ProjectSort,
};
use crate::shared::api::ApiResponse;
use crate::AppState;

//
// ──────────────────────────────────────────────────────────
// Query DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize)]
pub struct GetProjectsQuery {
    pub search: Option<String>,
    pub topic_id: Option<uuid::Uuid>,

    #[serde(default)]
    pub sort: ProjectSort,

    #[serde(default)]
    pub page: u32,

    #[serde(default)]
    pub per_page: u32,
}

impl From<GetProjectsQuery> for (ProjectListFilter, PageRequest, ProjectSort) {
    fn from(q: GetProjectsQuery) -> Self {
        let filter = ProjectListFilter {
            search: q.search,
            topic_id: q.topic_id,
        };

        let page = PageRequest {
            page: if q.page == 0 { 1 } else { q.page },
            per_page: if q.per_page == 0 { 10 } else { q.per_page },
        };

        (filter, page, q.sort)
    }
}

//
// ──────────────────────────────────────────────────────────
// Handler
// ──────────────────────────────────────────────────────────
//

#[get("/api/projects")]
pub async fn get_projects_handler(
    user: VerifiedUser,
    query: web::Query<GetProjectsQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let (filter, page, sort) = query.into_inner().into();

    match data
        .project
        .get_list
        .execute(owner, filter, sort, page)
        .await
    {
        Ok(result) => ApiResponse::success(result),

        Err(GetProjectsError::QueryFailed(msg)) => {
            error!("Failed to list projects: {}", msg);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::Value as JsonValue;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::domain::entities::UserId;
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::modules::project::application::ports::incoming::use_cases::{
        GetProjectsError, GetProjectsUseCase,
    };
    use crate::modules::project::application::ports::outgoing::project_query::{
        PageRequest, PageResult, ProjectCardView, ProjectListFilter, ProjectSort,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock GetProjects Use Case
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockGetProjectsUseCase {
        result: Result<PageResult<ProjectCardView>, GetProjectsError>,
    }

    impl MockGetProjectsUseCase {
        fn success(data: PageResult<ProjectCardView>) -> Self {
            Self { result: Ok(data) }
        }

        fn error(err: GetProjectsError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl GetProjectsUseCase for MockGetProjectsUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _filter: ProjectListFilter,
            _sort: ProjectSort,
            _page: PageRequest,
        ) -> Result<PageResult<ProjectCardView>, GetProjectsError> {
            self.result.clone()
        }
    }

    /* --------------------------------------------------
     * Helpers (JWT)
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

    fn sample_page_result() -> PageResult<ProjectCardView> {
        PageResult {
            items: vec![ProjectCardView {
                id: Uuid::new_v4(),
                title: "Test Project".to_string(),
                slug: "test-project".to_string(),
                tech_stack: vec!["Rust".to_string()],
                repo_url: Some("https://github.com/test/repo".to_string()),
                live_demo_url: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }],
            page: 1,
            per_page: 10,
            total: 1,
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_get_projects_success() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            // builder method name can stay descriptive;
            // internally it should set `state.project.get_list`
            .with_get_projects(MockGetProjectsUseCase::success(sample_page_result()))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_projects_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/projects")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: JsonValue = test::read_body_json(resp).await;

        assert_eq!(body["success"], true);
        assert!(body["error"].is_null());

        // Deserialize the data safely
        let page: PageResult<ProjectCardView> =
            serde_json::from_value(body["data"].clone()).unwrap();

        assert_eq!(page.total, 1);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].slug, "test-project");
    }

    #[actix_web::test]
    async fn test_get_projects_internal_error_on_query_failed() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_get_projects(MockGetProjectsUseCase::error(
                GetProjectsError::QueryFailed("db down".to_string()),
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_projects_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/projects")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn test_get_projects_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_get_projects(MockGetProjectsUseCase::success(sample_page_result()))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_projects_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/projects")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
