use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{header, header::HeaderValue, StatusCode},
    Error,
};
use futures::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::sync::Arc;

use super::policy::{client_key, limit_for};
use super::port::RateLimitStore;
use crate::shared::api::{ApiResponse, ErrorCode};

/// Applies per-caller limits to the unauthenticated auth endpoints.
///
/// Wraps the whole app and consults `policy::limit_for`, so an unlisted route
/// costs one string comparison and nothing else. That is preferred to wrapping
/// individual routes because the handlers are declared with attribute macros,
/// which cannot carry per-route middleware without restructuring them into
/// `web::resource` registrations.
pub struct RateLimit {
    store: Arc<dyn RateLimitStore>,
}

impl RateLimit {
    pub fn new(store: Arc<dyn RateLimitStore>) -> Self {
        Self { store }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service: Arc::new(service),
            store: Arc::clone(&self.store),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: Arc<S>,
    store: Arc<dyn RateLimitStore>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Arc::clone(&self.service);
        let store = Arc::clone(&self.store);

        Box::pin(async move {
            let Some(rule) = limit_for(req.method().as_str(), req.path()) else {
                return service
                    .call(req)
                    .await
                    .map(ServiceResponse::map_into_left_body);
            };

            let key = format!("{}:{}:{}", req.method(), req.path(), client_key(&req));

            match store.consume(&key, rule.limit, rule.window_secs).await {
                Ok(d) if !d.allowed => {
                    // Reuse ApiResponse so the body matches every other error
                    // the API emits, then attach Retry-After on top.
                    let mut response = ApiResponse::<()>::error(
                        StatusCode::TOO_MANY_REQUESTS,
                        ErrorCode::RateLimited,
                        "Too many requests. Please try again later.",
                    );

                    if let Ok(value) = HeaderValue::from_str(&d.retry_after_secs.to_string()) {
                        response.headers_mut().insert(header::RETRY_AFTER, value);
                    }

                    Ok(req.into_response(response).map_into_right_body())
                }

                Ok(_) => service
                    .call(req)
                    .await
                    .map(ServiceResponse::map_into_left_body),

                // Fail open. If Redis is unreachable, refusing every login would
                // turn a cache outage into a total authentication outage. The
                // limiter is a mitigation, not the security boundary, so
                // availability wins and the failure is logged.
                Err(e) => {
                    tracing::error!("Rate limit store unavailable, allowing request: {}", e);
                    service
                        .call(req)
                        .await
                        .map(ServiceResponse::map_into_left_body)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::rate_limit::port::{RateLimitDecision, RateLimitError};
    use actix_web::{test, web, App, HttpResponse};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory fixed-window counter. Keeps these tests off Redis, which the
    /// suite cannot reach, and makes the limit deterministic.
    #[derive(Default)]
    struct MemoryStore {
        counts: Mutex<HashMap<String, u32>>,
        fail: bool,
    }

    #[async_trait]
    impl RateLimitStore for MemoryStore {
        async fn consume(
            &self,
            key: &str,
            limit: u32,
            window_secs: u64,
        ) -> Result<RateLimitDecision, RateLimitError> {
            if self.fail {
                return Err(RateLimitError::Unavailable("down".into()));
            }
            let mut c = self.counts.lock().unwrap();
            let n = c.entry(key.to_string()).or_insert(0);
            *n += 1;
            Ok(RateLimitDecision {
                allowed: *n <= limit,
                remaining: limit.saturating_sub(*n),
                retry_after_secs: window_secs,
            })
        }
    }

    /// Builds the test app inline. A helper returning it would have to name
    /// `actix_http::Request`, which is not a direct dependency of this crate.
    macro_rules! app_with {
        ($store:expr) => {
            test::init_service(
                App::new()
                    .wrap(super::RateLimit::new($store))
                    .route(
                        "/api/auth/login",
                        web::post().to(|| async { HttpResponse::Ok().body("ok") }),
                    )
                    .route(
                        "/api/blog",
                        web::post().to(|| async { HttpResponse::Ok().body("ok") }),
                    ),
            )
            .await
        };
    }

    macro_rules! login_from {
        ($ip:expr) => {
            test::TestRequest::post()
                .uri("/api/auth/login")
                .insert_header(("x-forwarded-for", $ip))
                .to_request()
        };
    }

    #[actix_web::test]
    async fn allows_requests_under_the_limit() {
        let app = app_with!(Arc::new(MemoryStore::default()));

        // login is 10 per window
        for i in 0..10 {
            let resp = test::call_service(&app, login_from!("203.0.113.1")).await;
            assert_eq!(resp.status(), StatusCode::OK, "request {i} should pass");
        }
    }

    #[actix_web::test]
    async fn blocks_once_the_limit_is_exceeded() {
        let app = app_with!(Arc::new(MemoryStore::default()));

        for _ in 0..10 {
            test::call_service(&app, login_from!("203.0.113.2")).await;
        }

        let resp = test::call_service(&app, login_from!("203.0.113.2")).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

        // Retry-After tells the caller when to come back.
        assert!(resp.headers().contains_key(header::RETRY_AFTER));

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "RATE_LIMITED");
    }

    /// Counters are per caller. One client exhausting its quota must not lock
    /// out everyone else, which is exactly what would happen if the key were the
    /// load balancer's address.
    #[actix_web::test]
    async fn one_caller_hitting_the_limit_does_not_affect_another() {
        let app = app_with!(Arc::new(MemoryStore::default()));

        for _ in 0..11 {
            test::call_service(&app, login_from!("203.0.113.3")).await;
        }
        let blocked = test::call_service(&app, login_from!("203.0.113.3")).await;
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);

        let other = test::call_service(&app, login_from!("203.0.113.4")).await;
        assert_eq!(other.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn unlisted_routes_are_never_limited() {
        let app = app_with!(Arc::new(MemoryStore::default()));

        for _ in 0..50 {
            let req = test::TestRequest::post()
                .uri("/api/blog")
                .insert_header(("x-forwarded-for", "203.0.113.5"))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);
        }
    }

    /// If Redis is down, refusing every login would turn a cache outage into a
    /// total authentication outage. The limiter is a mitigation, not the
    /// security boundary, so it fails open.
    #[actix_web::test]
    async fn fails_open_when_the_store_is_unavailable() {
        let app = app_with!(Arc::new(MemoryStore {
            fail: true,
            ..Default::default()
        }));

        for _ in 0..20 {
            let resp = test::call_service(&app, login_from!("203.0.113.6")).await;
            assert_eq!(resp.status(), StatusCode::OK);
        }
    }
}
