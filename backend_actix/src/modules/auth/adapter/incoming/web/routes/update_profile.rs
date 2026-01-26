use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::AuthenticatedUser,
        application::{
            domain::entities::UserId,
            use_cases::update_profile::{UpdateUserError, UpdateUserInput},
        },
    },
    AppState,
};
use actix_web::{put, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Deserialize)]
struct UpdateUserRequest {
    full_name: String,
}

#[derive(Serialize)]
struct UpdateUserResponse {
    user_id: String,
    email: String,
    username: String,
    full_name: String,
}

#[put("/api/users/me")]
pub async fn update_user_profile_handler(
    user: AuthenticatedUser,
    req: web::Json<UpdateUserRequest>,
    app_data: web::Data<AppState>,
) -> impl Responder {
    let input = UpdateUserInput {
        user_id: UserId::from(user.user_id),
        full_name: req.into_inner().full_name,
    };

    match app_data.update_user_profile_use_case.execute(input).await {
        Ok(output) => HttpResponse::Ok().json(UpdateUserResponse {
            user_id: output.user_id.value().to_string(),
            email: output.email,
            username: output.username,
            full_name: output.full_name,
        }),
        Err(UpdateUserError::InvalidFullName(msg)) => {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid full name",
                "message": msg
            }))
        }

        Err(UpdateUserError::RepositoryError(e)) => {
            error!("Repository error updating user profile: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
        Err(UpdateUserError::QueryError(e)) => {
            error!("Query error updating user profile: {}", e);
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
            ports::outgoing::{
                token_provider::{TokenClaims, TokenError, TokenProvider},
                user_repository::UserRepositoryError,
            },
            use_cases::update_profile::{
                UpdateUserError, UpdateUserInput, UpdateUserOutput, UpdateUserProfileUseCase,
            },
        },
        tests::support::app_state_builder::TestAppStateBuilder,
    };
    use actix_web::{test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use uuid::Uuid;

    struct MockUpdateUserProfileUseCase {
        result: Result<UpdateUserOutput, UpdateUserError>,
    }

    #[async_trait]
    impl UpdateUserProfileUseCase for MockUpdateUserProfileUseCase {
        async fn execute(
            &self,
            _data: UpdateUserInput,
        ) -> Result<UpdateUserOutput, UpdateUserError> {
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

    fn create_update_user_output(user_id: Uuid, full_name: &str) -> UpdateUserOutput {
        UpdateUserOutput {
            user_id: user_id.into(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            full_name: full_name.to_string(),
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
    async fn test_update_user_profile_success() {
        let user_id = Uuid::new_v4();
        let new_full_name = "Updated Name";

        let mock_use_case = MockUpdateUserProfileUseCase {
            result: Ok(create_update_user_output(user_id, new_full_name)),
        };

        let app_state = TestAppStateBuilder::default()
            .with_update_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(update_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .insert_header(("Content-Type", "application/json"))
            .set_json(serde_json::json!({
                "full_name": new_full_name
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["user_id"], user_id.to_string());
        assert_eq!(body["email"], "test@example.com");
        assert_eq!(body["username"], "testuser");
        assert_eq!(body["full_name"], new_full_name);
    }

    #[actix_web::test]
    async fn test_update_user_profile_invalid_full_name() {
        let user_id = Uuid::new_v4();

        let mock_use_case = MockUpdateUserProfileUseCase {
            result: Err(UpdateUserError::InvalidFullName(
                "Full name must be 2-100 characters".to_string(),
            )),
        };

        let app_state = TestAppStateBuilder::default()
            .with_update_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(update_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .insert_header(("Content-Type", "application/json"))
            .set_json(serde_json::json!({
                "full_name": "A"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Invalid full name");
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("2-100 characters"));
    }

    #[actix_web::test]
    async fn test_update_user_profile_repository_error() {
        let user_id = Uuid::new_v4();

        let mock_use_case = MockUpdateUserProfileUseCase {
            result: Err(UpdateUserError::RepositoryError(
                UserRepositoryError::DatabaseError("Connection failed".to_string()),
            )),
        };

        let app_state = TestAppStateBuilder::default()
            .with_update_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(update_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .insert_header(("Content-Type", "application/json"))
            .set_json(serde_json::json!({
                "full_name": "Valid Name"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Internal server error");
        assert!(body.get("message").is_none());
    }

    #[actix_web::test]
    async fn test_update_user_profile_query_error() {
        let user_id = Uuid::new_v4();

        let mock_use_case = MockUpdateUserProfileUseCase {
            result: Err(UpdateUserError::QueryError(
                crate::auth::application::ports::outgoing::user_query::UserQueryError::DatabaseError(
                    "Query failed".to_string(),
                ),
            )),
        };

        let app_state = TestAppStateBuilder::default()
            .with_update_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(update_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .insert_header(("Content-Type", "application/json"))
            .set_json(serde_json::json!({
                "full_name": "Valid Name"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Internal server error");
        assert!(body.get("message").is_none());
    }
}
