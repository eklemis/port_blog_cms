use actix_web::{get, web, Responder};
use serde::Deserialize;
use tracing::error;

use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::resolve_owner_id_or_response,
        application::domain::entities::UserId,
    },
    modules::project::application::ports::incoming::use_cases::GetPublicSingleProjectError,
    shared::api::ApiResponse,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct PublicProjectPath {
    pub username: String,
    pub project_slug: String,
}

#[get("/api/public/projects/{username}/{project_slug}")]
pub async fn get_public_single_project_handler(
    path: web::Path<PublicProjectPath>,
    data: web::Data<AppState>,
) -> impl Responder {
    let path = path.into_inner();

    // Resolve owner_id from username
    let owner_id = match resolve_owner_id_or_response(&data, &path.username).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Use case: owner + slug -> ProjectView
    match data
        .project
        .get_public_single
        .execute(UserId::from(owner_id), &path.project_slug)
        .await
    {
        Ok(project) => ApiResponse::success(project),

        Err(GetPublicSingleProjectError::NotFound) => {
            ApiResponse::not_found("PROJECT_NOT_FOUND", "Project not found")
        }

        Err(GetPublicSingleProjectError::RepositoryError(msg)) => {
            error!(
                "Repository error fetching public project slug={} for username={}: {}",
                path.project_slug, path.username, msg
            );
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

    use crate::auth::application::domain::entities::UserId;

    use crate::modules::project::application::ports::incoming::use_cases::{
        GetPublicSingleProjectError, GetPublicSingleProjectUseCase,
    };
    use crate::modules::project::application::ports::outgoing::project_query::ProjectView;

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
            unimplemented!("not used in public single project route tests")
        }

        async fn find_by_email(
            &self,
            _email: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!("not used in public single project route tests")
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
     * Mock GetPublicSingleProjectUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockGetPublicSingleProjectUseCase {
        result: Result<ProjectView, GetPublicSingleProjectError>,
    }

    impl MockGetPublicSingleProjectUseCase {
        fn success(view: ProjectView) -> Self {
            Self { result: Ok(view) }
        }

        fn error(err: GetPublicSingleProjectError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl GetPublicSingleProjectUseCase for MockGetPublicSingleProjectUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _slug: &str,
        ) -> Result<ProjectView, GetPublicSingleProjectError> {
            self.result.clone()
        }
    }

    fn sample_project_view(owner: UserId, slug: &str) -> ProjectView {
        ProjectView {
            id: Uuid::new_v4(),
            owner,
            title: "Public Project".to_string(),
            slug: slug.to_string(),
            description: "desc".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: vec!["img.png".to_string()],
            repo_url: None,
            live_demo_url: None,
            topics: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_get_public_single_project_success() {
        let owner_uuid = Uuid::new_v4();
        let username = "someone";
        let project_slug = "public-project";

        let user_query =
            MockUserQuery::found(sample_user_query_result(owner_uuid, username, false));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let view = sample_project_view(UserId::from(owner_uuid), project_slug);

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_project(MockGetPublicSingleProjectUseCase::success(view))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_single_project_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/public/projects/{}/{}",
                username, project_slug
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;

        // Envelope
        assert_eq!(body["success"], true);
        assert!(body["error"].is_null());

        // Shape checks
        assert!(body["data"].is_object());
        assert!(body["data"]["id"].is_string());
        assert_eq!(body["data"]["slug"], project_slug);
        assert_eq!(body["data"]["title"], "Public Project");
        assert!(body["data"]["tech_stack"].is_array());
        assert!(body["data"]["screenshots"].is_array());
        assert!(body["data"]["created_at"].is_string());
        assert!(body["data"]["updated_at"].is_string());

        // owner shape: just ensure present and not null
        assert!(body["data"].get("owner").is_some());
        assert!(!body["data"]["owner"].is_null());
    }

    #[actix_web::test]
    async fn test_get_public_single_project_user_not_found() {
        let username = "missing-user";
        let project_slug = "public-project";

        let resolver = UserIdentityResolver::new(Arc::new(MockUserQuery::not_found()));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            // shouldn't be called, but still set something safe
            .with_get_public_single_project(MockGetPublicSingleProjectUseCase::error(
                GetPublicSingleProjectError::NotFound,
            ))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_single_project_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/public/projects/{}/{}",
                username, project_slug
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "USER_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_public_single_project_deleted_user_treated_as_not_found() {
        let owner_uuid = Uuid::new_v4();
        let username = "deleted-user";
        let project_slug = "public-project";

        let user_query = MockUserQuery::found(sample_user_query_result(owner_uuid, username, true));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_project(MockGetPublicSingleProjectUseCase::error(
                GetPublicSingleProjectError::NotFound,
            ))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_single_project_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/public/projects/{}/{}",
                username, project_slug
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "USER_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_public_single_project_project_not_found() {
        let owner_uuid = Uuid::new_v4();
        let username = "someone";
        let project_slug = "missing-slug";

        let user_query =
            MockUserQuery::found(sample_user_query_result(owner_uuid, username, false));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_project(MockGetPublicSingleProjectUseCase::error(
                GetPublicSingleProjectError::NotFound,
            ))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_single_project_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/public/projects/{}/{}",
                username, project_slug
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_public_single_project_resolver_repository_error_internal_error() {
        let username = "someone";
        let project_slug = "public-project";

        let user_query = MockUserQuery::error(UserQueryError::DatabaseError("db down".to_string()));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_project(MockGetPublicSingleProjectUseCase::error(
                GetPublicSingleProjectError::NotFound,
            ))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_single_project_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/public/projects/{}/{}",
                username, project_slug
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_get_public_single_project_use_case_repository_error_internal_error() {
        let owner_uuid = Uuid::new_v4();
        let username = "someone";
        let project_slug = "public-project";

        let user_query =
            MockUserQuery::found(sample_user_query_result(owner_uuid, username, false));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_project(MockGetPublicSingleProjectUseCase::error(
                GetPublicSingleProjectError::RepositoryError("db down".to_string()),
            ))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_single_project_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/public/projects/{}/{}",
                username, project_slug
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }
}
