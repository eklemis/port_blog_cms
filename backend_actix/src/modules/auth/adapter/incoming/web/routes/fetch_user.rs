use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::AuthenticatedUser,
        application::{domain::entities::UserId, use_cases::fetch_profile::FetchUserError},
    },
    AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;
use tracing::error;

#[derive(Serialize)]
struct UserProfileResponse {
    user_id: String,
    email: String,
    username: String,
    full_name: String,
}

#[get("/api/users/me")]
pub async fn get_user_profile_handler(
    user: AuthenticatedUser,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .fetch_user_profile_use_case
        .execute(UserId::from(user.user_id))
        .await
    {
        Ok(user_output) => HttpResponse::Ok().json(UserProfileResponse {
            user_id: user_output.user_id.value().to_string(),
            email: user_output.email,
            username: user_output.username,
            full_name: user_output.full_name,
        }),
        Err(FetchUserError::UserNotFound(msg)) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "User not found",
                "message": msg
            }))
        }

        Err(FetchUserError::QueryError(e)) => {
            error!("Database error fetching user profile: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::application::{
            domain::entities::UserId,
            ports::outgoing::{
                token_provider::{TokenClaims, TokenError, TokenProvider},
                user_query::UserQueryError,
            },
            use_cases::fetch_profile::{FetchUserError, FetchUserOutput, FetchUserProfileUseCase},
        },
        tests::support::app_state_builder::TestAppStateBuilder,
    };
    use actix_web::{test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use uuid::Uuid;

    struct MockFetchUserProfileUseCase {
        result: Result<FetchUserOutput, FetchUserError>,
    }

    #[async_trait]
    impl FetchUserProfileUseCase for MockFetchUserProfileUseCase {
        async fn execute(&self, _user_id: UserId) -> Result<FetchUserOutput, FetchUserError> {
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

    fn create_fetch_user_output(user_id: Uuid) -> FetchUserOutput {
        FetchUserOutput {
            user_id: user_id.into(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            full_name: "Test User".to_string(),
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
    async fn test_get_user_profile_success() {
        let user_id = Uuid::new_v4();
        let mock_use_case = MockFetchUserProfileUseCase {
            result: Ok(create_fetch_user_output(user_id)),
        };

        let app_state = TestAppStateBuilder::default()
            .with_fetch_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(get_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["user_id"], user_id.to_string());
        assert_eq!(body["email"], "test@example.com");
        assert_eq!(body["username"], "testuser");
        assert_eq!(body["full_name"], "Test User");
    }

    #[actix_web::test]
    async fn test_get_user_profile_success_unverified_user() {
        let user_id = Uuid::new_v4();
        let mock_use_case = MockFetchUserProfileUseCase {
            result: Ok(create_fetch_user_output(user_id)),
        };

        let app_state = TestAppStateBuilder::default()
            .with_fetch_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, false);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(get_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["user_id"], user_id.to_string());
    }

    #[actix_web::test]
    async fn test_get_user_profile_not_found() {
        let user_id = Uuid::new_v4();
        let mock_use_case = MockFetchUserProfileUseCase {
            result: Err(FetchUserError::UserNotFound(user_id.to_string())),
        };

        let app_state = TestAppStateBuilder::default()
            .with_fetch_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(get_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 404);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "User not found");
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains(&user_id.to_string()));
    }

    #[actix_web::test]
    async fn test_get_user_profile_query_error() {
        let user_id = Uuid::new_v4();
        let mock_use_case = MockFetchUserProfileUseCase {
            result: Err(FetchUserError::QueryError(UserQueryError::DatabaseError(
                "Connection failed".to_string(),
            ))),
        };

        let app_state = TestAppStateBuilder::default()
            .with_fetch_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(get_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Internal server error");
        assert!(body.get("message").is_none());
    }
}
