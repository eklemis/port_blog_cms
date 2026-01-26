use actix_web::{delete, web, HttpResponse, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    cv::application::use_cases::hard_delete_cv::HardDeleteCVError,
    AppState,
};

#[delete("/api/cvs/{cv_id}")]
pub async fn hard_delete_cv_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    app_data: web::Data<AppState>,
) -> impl Responder {
    let cv_id = path.into_inner();

    match app_data
        .hard_delete_cv_use_case
        .execute(UserId::from(user.user_id), cv_id)
        .await
    {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(HardDeleteCVError::CVNotFound) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "CV not found"
        })),
        Err(HardDeleteCVError::Unauthorized) => HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Unauthorized",
            "message": "You are not authorized to delete this CV"
        })),
        Err(HardDeleteCVError::RepositoryError(e)) => {
            error!("Repository error deleting CV: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::use_cases::hard_delete_cv::{
        HardDeleteCVError, HardDeleteCvUseCase,
    };
    use crate::{
        auth::application::{
            domain::entities::UserId,
            ports::outgoing::token_provider::{TokenClaims, TokenError, TokenProvider},
        },
        tests::support::app_state_builder::TestAppStateBuilder,
    };
    use actix_web::{test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use uuid::Uuid;

    struct MockHardDeleteCvUseCase {
        result: Result<(), HardDeleteCVError>,
    }

    #[async_trait]
    impl HardDeleteCvUseCase for MockHardDeleteCvUseCase {
        async fn execute(&self, _user_id: UserId, _cv_id: Uuid) -> Result<(), HardDeleteCVError> {
            self.result.clone()
        }
    }

    struct MockTokenProvider {
        user_id: Uuid,
        is_verified: bool,
    }

    impl TokenProvider for MockTokenProvider {
        fn generate_access_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!()
        }

        fn generate_refresh_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!()
        }

        fn verify_token(&self, _token: &str) -> Result<TokenClaims, TokenError> {
            Ok(TokenClaims {
                sub: self.user_id,
                token_type: "access".to_string(),
                is_verified: self.is_verified,
                exp: 9999999999,
                iat: 0,
                nbf: 0,
            })
        }

        fn refresh_access_token(&self, _refresh_token: &str) -> Result<String, TokenError> {
            unimplemented!()
        }

        fn generate_verification_token(&self, _user_id: Uuid) -> Result<String, TokenError> {
            unimplemented!()
        }

        fn verify_verification_token(&self, _token: &str) -> Result<Uuid, TokenError> {
            unimplemented!()
        }
    }

    fn create_token_provider(
        user_id: Uuid,
        is_verified: bool,
    ) -> web::Data<Arc<dyn TokenProvider + Send + Sync>> {
        web::Data::new(Arc::new(MockTokenProvider {
            user_id,
            is_verified,
        }) as Arc<dyn TokenProvider + Send + Sync>)
    }

    #[actix_web::test]
    async fn test_hard_delete_cv_success() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let mock_use_case = MockHardDeleteCvUseCase { result: Ok(()) };

        let app_state = TestAppStateBuilder::default()
            .with_hard_delete_cv(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(hard_delete_cv_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 204);
    }

    #[actix_web::test]
    async fn test_hard_delete_cv_not_found() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let mock_use_case = MockHardDeleteCvUseCase {
            result: Err(HardDeleteCVError::CVNotFound),
        };

        let app_state = TestAppStateBuilder::default()
            .with_hard_delete_cv(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(hard_delete_cv_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 404);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "CV not found");
    }

    #[actix_web::test]
    async fn test_hard_delete_cv_unauthorized() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let mock_use_case = MockHardDeleteCvUseCase {
            result: Err(HardDeleteCVError::Unauthorized),
        };

        let app_state = TestAppStateBuilder::default()
            .with_hard_delete_cv(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(hard_delete_cv_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 403);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Unauthorized");
        assert!(body["message"].as_str().unwrap().contains("not authorized"));
    }

    #[actix_web::test]
    async fn test_hard_delete_cv_repository_error() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let mock_use_case = MockHardDeleteCvUseCase {
            result: Err(HardDeleteCVError::RepositoryError(
                "Database connection failed".to_string(),
            )),
        };

        let app_state = TestAppStateBuilder::default()
            .with_hard_delete_cv(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(hard_delete_cv_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Internal server error");
        assert!(body.get("message").is_none());
    }
}
