use actix_web::{get, web, Responder};
use tracing::error;

use crate::auth::adapter::incoming::web::extractors::auth::resolve_owner_id_or_response;
use crate::auth::application::domain::entities::UserId;
use crate::auth::application::helpers::ResolveUserIdError;
use crate::modules::project::adapter::incoming::web::routes::get_projects::GetProjectsQuery;
use crate::modules::project::application::ports::incoming::use_cases::GetProjectsError;
use crate::shared::api::ApiResponse;
use crate::AppState;

//
// ──────────────────────────────────────────────────────────
// Handler
// ──────────────────────────────────────────────────────────
//

#[get("/api/public/projects/{username}")]
pub async fn get_public_projects_handler(
    path: web::Path<String>,
    query: web::Query<GetProjectsQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let username = path.into_inner();
    let (filter, page, sort) = query.into_inner().into();

    // 1. Resolve owner_id from username
    let owner_id = match resolve_owner_id_or_response(&data, &username).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // 2. Delegate to existing use case
    match data
        .project
        .get_list
        .execute(UserId::from(owner_id), filter, sort, page)
        .await
    {
        Ok(result) => ApiResponse::success(result),

        Err(GetProjectsError::QueryFailed(msg)) => {
            error!("Failed to list public projects: {}", msg);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::application::helpers::UserIdentityResolver;
    use crate::auth::application::ports::outgoing::user_query::{
        UserQuery, UserQueryError, UserQueryResult,
    };

    use crate::modules::project::application::ports::incoming::use_cases::{
        GetProjectsError, GetProjectsUseCase,
    };
    use crate::modules::project::application::ports::outgoing::project_query::{
        PageRequest, PageResult, ProjectCardView, ProjectListFilter, ProjectSort,
    };

    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock UserQuery (drives UserIdentityResolver)
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockUserQuery {
        result: Result<Option<UserQueryResult>, UserQueryError>,
    }

    impl MockUserQuery {
        fn found(user: UserQueryResult) -> Self {
            Self {
                result: Ok(Some(user)),
            }
        }

        fn not_found() -> Self {
            Self { result: Ok(None) }
        }

        fn error(err: UserQueryError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_id(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!("not used in public projects route tests")
        }

        async fn find_by_email(
            &self,
            _email: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!("not used in public projects route tests")
        }

        async fn find_by_username(
            &self,
            _username: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            self.result.clone()
        }
    }

    fn sample_user_query_result(id: Uuid, username: &str, is_deleted: bool) -> UserQueryResult {
        UserQueryResult {
            id,
            email: "test@example.com".to_string(),
            username: username.to_string(),
            password_hash: "hashed".to_string(),
            full_name: "Test User".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_verified: true,
            is_deleted,
        }
    }

    /* --------------------------------------------------
     * Mock GetProjectsUseCase
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
            _owner: crate::auth::application::domain::entities::UserId,
            _filter: ProjectListFilter,
            _sort: ProjectSort,
            _page: PageRequest,
        ) -> Result<PageResult<ProjectCardView>, GetProjectsError> {
            self.result.clone()
        }
    }

    fn sample_page_result() -> PageResult<ProjectCardView> {
        PageResult {
            items: vec![ProjectCardView {
                id: Uuid::new_v4(),
                title: "Public Project".to_string(),
                slug: "public-project".to_string(),
                tech_stack: vec!["Rust".to_string()],
                repo_url: None,
                live_demo_url: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
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
    async fn test_get_public_projects_success() {
        let owner_uuid = Uuid::new_v4();
        let username = "someone";

        let user_query =
            MockUserQuery::found(sample_user_query_result(owner_uuid, username, false));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_get_projects(MockGetProjectsUseCase::success(sample_page_result()))
            .with_user_identity_resolver(resolver)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_projects_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/projects/{}", username))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;

        // Envelope
        assert_eq!(body["success"], true);
        assert!(body["error"].is_null());

        // Shape checks
        assert!(body["data"].is_object());
        assert!(body["data"]["items"].is_array());
        assert!(body["data"]["page"].is_number());
        assert!(body["data"]["per_page"].is_number());
        assert!(body["data"]["total"].is_number());

        // Item shape (minimal)
        assert!(body["data"]["items"][0]["id"].is_string());
        assert_eq!(body["data"]["items"][0]["slug"], "public-project");
        assert!(body["data"]["items"][0]["tech_stack"].is_array());
    }

    #[actix_web::test]
    async fn test_get_public_projects_user_not_found() {
        let username = "missing-user";

        let user_query = MockUserQuery::not_found();
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_get_projects(MockGetProjectsUseCase::success(sample_page_result()))
            .with_user_identity_resolver(resolver)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_projects_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/projects/{}", username))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;

        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "USER_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_public_projects_deleted_user_treated_as_not_found() {
        let owner_uuid = Uuid::new_v4();
        let username = "deleted-user";

        let user_query = MockUserQuery::found(sample_user_query_result(owner_uuid, username, true));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_get_projects(MockGetProjectsUseCase::success(sample_page_result()))
            .with_user_identity_resolver(resolver)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_projects_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/projects/{}", username))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;

        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "USER_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_public_projects_resolver_repository_error_internal_error() {
        let username = "someone";

        let user_query = MockUserQuery::error(UserQueryError::DatabaseError("db down".to_string()));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_get_projects(MockGetProjectsUseCase::success(sample_page_result()))
            .with_user_identity_resolver(resolver)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_projects_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/projects/{}", username))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;

        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_get_public_projects_use_case_error_internal_error() {
        let owner_uuid = Uuid::new_v4();
        let username = "someone";

        let user_query =
            MockUserQuery::found(sample_user_query_result(owner_uuid, username, false));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_get_projects(MockGetProjectsUseCase::error(
                GetProjectsError::QueryFailed("db down".to_string()),
            ))
            .with_user_identity_resolver(resolver)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_projects_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/projects/{}", username))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;

        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }
}
