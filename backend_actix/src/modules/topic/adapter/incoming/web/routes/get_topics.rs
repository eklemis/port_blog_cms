use actix_web::{get, web, Responder};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    shared::api::ApiResponse,
    topic::application::ports::incoming::use_cases::GetTopicsError,
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TopicResponse {
    title: String,
    description: String,
}

#[get("/api/topics")]
pub async fn get_topics_handler(user: VerifiedUser, data: web::Data<AppState>) -> impl Responder {
    let owner = user.user_id.clone();

    match data.get_topics_use_case.execute(UserId::from(owner)).await {
        Ok(topics) => {
            let response = topics
                .into_iter()
                .map(|topic| TopicResponse {
                    title: topic.title,
                    description: topic.description,
                })
                .collect::<Vec<_>>();

            ApiResponse::success(response)
        }

        Err(err) => map_get_topics_error(err),
    }
}

fn map_get_topics_error(err: GetTopicsError) -> actix_web::HttpResponse {
    match err {
        GetTopicsError::QueryFailed(_) => ApiResponse::internal_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, web, App};
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        auth::application::domain::entities::UserId,
        auth::application::ports::outgoing::token_provider::{
            TokenClaims, TokenError, TokenProvider,
        },
        tests::support::{app_state_builder::TestAppStateBuilder, stubs::StubGetTopicsUseCase},
        topic::application::ports::outgoing::TopicQueryResult,
    };

    // ============================================================
    // TokenProvider Stub (FULL, trait-accurate)
    // ============================================================

    #[derive(Clone)]
    struct StubTokenProvider {
        user_id: Uuid,
    }

    impl TokenProvider for StubTokenProvider {
        fn generate_access_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!("Not used in get_topics tests")
        }

        fn generate_refresh_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!("Not used in get_topics tests")
        }

        fn verify_token(&self, _token: &str) -> Result<TokenClaims, TokenError> {
            Ok(TokenClaims {
                sub: self.user_id,
                exp: 9_999_999_999,
                iat: 0,
                nbf: 0,
                token_type: "access".to_string(),
                is_verified: true,
            })
        }

        fn refresh_access_token(&self, _refresh_token: &str) -> Result<String, TokenError> {
            unimplemented!("Not used in get_topics tests")
        }

        fn generate_verification_token(&self, _user_id: Uuid) -> Result<String, TokenError> {
            unimplemented!("Not used in get_topics tests")
        }

        fn verify_verification_token(&self, _token: &str) -> Result<Uuid, TokenError> {
            unimplemented!("Not used in get_topics tests")
        }
    }

    // ============================================================
    // Helpers
    // ============================================================

    fn topic(owner: UserId, title: &str) -> TopicQueryResult {
        TopicQueryResult {
            id: Uuid::new_v4(),
            owner,
            title: title.to_string(),
            description: "desc".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    async fn read_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
        let body = test::read_body(resp).await;
        serde_json::from_slice(&body).unwrap()
    }

    // ============================================================
    // Tests
    // ============================================================

    #[actix_web::test]
    async fn get_topics_success_with_results() {
        // Arrange
        let user_id = Uuid::new_v4();
        let owner = UserId::from(user_id);

        let topics = vec![
            topic(owner.clone(), "Rust"),
            topic(owner.clone(), "Backend"),
        ];

        let state = TestAppStateBuilder::default()
            .with_get_topics(StubGetTopicsUseCase::success(topics))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> =
            Arc::new(StubTokenProvider { user_id });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(get_topics_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/topics")
            .insert_header(("Authorization", "Bearer test-token"))
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::OK);

        let json = read_json(resp).await;
        assert_eq!(json["success"], true);
        assert_eq!(json["data"].as_array().unwrap().len(), 2);
    }

    #[actix_web::test]
    async fn get_topics_query_failure_returns_internal_error() {
        // Arrange
        let user_id = Uuid::new_v4();

        let state = TestAppStateBuilder::default()
            .with_get_topics(StubGetTopicsUseCase::failure("db down"))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> =
            Arc::new(StubTokenProvider { user_id });

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(token_provider))
                .service(get_topics_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/topics")
            .insert_header(("Authorization", "Bearer test-token"))
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let json = read_json(resp).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "INTERNAL_ERROR");
    }
}
