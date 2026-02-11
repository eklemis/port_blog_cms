use actix_web::{get, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::adapter::incoming::web::extractors::auth::resolve_owner_id_or_response,
    cv::application::use_cases::get_public_single_cv::GetPublicSingleCvError,
    shared::api::ApiResponse, AppState,
};

#[get("/api/public/cvs/{username}/{cv_id}")]
pub async fn get_public_cv_by_id_handler(
    path: web::Path<(String, Uuid)>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (username, cv_id) = path.into_inner();

    // Resolve owner_id from username
    let owner_id = match resolve_owner_id_or_response(&data, &username).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Fetch the CV publicly (still owner-scoped)
    match data
        .get_public_single_cv_use_case
        .execute(owner_id.into(), cv_id)
        .await
    {
        Ok(cv) => ApiResponse::success(cv),

        Err(GetPublicSingleCvError::NotFound) => {
            ApiResponse::not_found("CV_NOT_FOUND", "CV not found")
        }

        Err(GetPublicSingleCvError::RepositoryError(msg)) => {
            error!(
                "Repository error fetching public CV {} for username {}: {}",
                cv_id, username, msg
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

    use crate::cv::application::use_cases::get_public_single_cv::{
        GetPublicSingleCvError, GetPublicSingleCvUseCase,
    };
    use crate::cv::domain::entities::CVInfo;

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
            unimplemented!("not used in public CV route tests")
        }

        async fn find_by_email(
            &self,
            _email: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!("not used in public CV route tests")
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
     * Mock GetPublicSingleCvUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockGetPublicSingleCvUseCase {
        result: Result<CVInfo, GetPublicSingleCvError>,
    }

    impl MockGetPublicSingleCvUseCase {
        fn success(cv: CVInfo) -> Self {
            Self { result: Ok(cv) }
        }

        fn not_found() -> Self {
            Self {
                result: Err(GetPublicSingleCvError::NotFound),
            }
        }

        fn error(msg: &str) -> Self {
            Self {
                result: Err(GetPublicSingleCvError::RepositoryError(msg.to_string())),
            }
        }
    }

    #[async_trait]
    impl GetPublicSingleCvUseCase for MockGetPublicSingleCvUseCase {
        async fn execute(
            &self,
            _owner_id: Uuid,
            _cv_id: Uuid,
        ) -> Result<CVInfo, GetPublicSingleCvError> {
            self.result.clone()
        }
    }

    fn sample_cv(owner_id: Uuid, cv_id: Uuid) -> CVInfo {
        CVInfo {
            id: cv_id,
            user_id: owner_id,
            display_name: "Public CV".to_string(),
            role: "Engineer".to_string(),
            bio: "Hello".to_string(),
            photo_url: "".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_get_public_cv_by_id_success() {
        let owner_uuid = Uuid::new_v4();
        let cv_id = Uuid::new_v4();
        let username = "someone";

        let user_query =
            MockUserQuery::found(sample_user_query_result(owner_uuid, username, false));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let cv = sample_cv(owner_uuid, cv_id);

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_cv(Arc::new(MockGetPublicSingleCvUseCase::success(cv)))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/cvs/{}/{}", username, cv_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;

        // Envelope
        assert_eq!(body["success"], true);
        assert!(body["error"].is_null());

        // Shape checks (CVInfo is complex; we only assert key fields)
        assert!(body["data"].is_object());
        assert_eq!(body["data"]["id"].as_str().unwrap(), cv_id.to_string());
        assert!(body["data"]["user_id"].is_string());
        assert_eq!(body["data"]["display_name"], "Public CV");
        assert!(body["data"]["experiences"].is_array());
    }

    #[actix_web::test]
    async fn test_get_public_cv_by_id_user_not_found() {
        let cv_id = Uuid::new_v4();
        let username = "missing-user";

        let user_query = MockUserQuery::not_found();
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_cv(Arc::new(MockGetPublicSingleCvUseCase::not_found()))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/cvs/{}/{}", username, cv_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "USER_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_public_cv_by_id_deleted_user_treated_as_not_found() {
        let owner_uuid = Uuid::new_v4();
        let cv_id = Uuid::new_v4();
        let username = "deleted-user";

        let user_query = MockUserQuery::found(sample_user_query_result(owner_uuid, username, true));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_cv(Arc::new(MockGetPublicSingleCvUseCase::not_found()))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/cvs/{}/{}", username, cv_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "USER_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_public_cv_by_id_resolver_repository_error_internal_error() {
        let cv_id = Uuid::new_v4();
        let username = "someone";

        let user_query = MockUserQuery::error(UserQueryError::DatabaseError("db down".to_string()));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_cv(Arc::new(MockGetPublicSingleCvUseCase::not_found()))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/cvs/{}/{}", username, cv_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_get_public_cv_by_id_cv_not_found() {
        let owner_uuid = Uuid::new_v4();
        let cv_id = Uuid::new_v4();
        let username = "someone";

        let user_query =
            MockUserQuery::found(sample_user_query_result(owner_uuid, username, false));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_cv(Arc::new(MockGetPublicSingleCvUseCase::not_found()))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/cvs/{}/{}", username, cv_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "CV_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_get_public_cv_by_id_use_case_repo_error_internal_error() {
        let owner_uuid = Uuid::new_v4();
        let cv_id = Uuid::new_v4();
        let username = "someone";

        let user_query =
            MockUserQuery::found(sample_user_query_result(owner_uuid, username, false));
        let resolver = UserIdentityResolver::new(Arc::new(user_query));

        let app_state = TestAppStateBuilder::default()
            .with_user_identity_resolver(resolver)
            .with_get_public_single_cv(Arc::new(MockGetPublicSingleCvUseCase::error("db down")))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(get_public_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/cvs/{}/{}", username, cv_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }
}
