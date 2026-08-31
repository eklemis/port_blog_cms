use actix_web::dev::ServiceRequest;

/// A limit applied to one route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimit {
    pub limit: u32,
    pub window_secs: u64,
}

/// Which routes are limited, and how hard.
///
/// Only the unauthenticated auth endpoints are covered. Everything else already
/// requires a valid token, so abusing it means holding a credential — a
/// different problem from the one this solves.
///
/// The numbers are deliberately low because each of these is expensive on our
/// side: login and password reset run Argon2, and registration runs Argon2 and
/// sends mail. That makes them a denial-of-service lever as much as a
/// credential-guessing one.
pub fn limit_for(method: &str, path: &str) -> Option<RateLimit> {
    if method != "POST" {
        return None;
    }

    match path {
        "/api/auth/login" => Some(RateLimit {
            limit: 10,
            window_secs: 300,
        }),
        "/api/auth/register" => Some(RateLimit {
            limit: 5,
            window_secs: 3600,
        }),
        "/api/auth/password-reset" => Some(RateLimit {
            limit: 5,
            window_secs: 3600,
        }),
        "/api/auth/refresh" => Some(RateLimit {
            limit: 30,
            window_secs: 300,
        }),
        // The completion endpoint carries a token in the path, so it is matched
        // by prefix rather than equality. It is limited because the token is the
        // only thing standing between a caller and a password change.
        p if p.starts_with("/api/auth/password-reset/") => Some(RateLimit {
            limit: 10,
            window_secs: 3600,
        }),
        _ => None,
    }
}

/// Identifies the caller for limiting purposes.
///
/// Behind Cloud Run every request arrives from the load balancer, so
/// `peer_addr` is the proxy for all callers. Using it would collapse every
/// client onto one counter, which turns the limiter into a self-inflicted
/// outage: one busy client locks out everybody.
///
/// `X-Forwarded-For` is therefore preferred, taking the left-most entry — the
/// original client. That header is caller-supplied and trivially spoofed, so
/// this is only sound because Cloud Run overwrites it; behind a proxy that
/// merely appends, an attacker could rotate the value to get a fresh bucket per
/// request.
pub fn client_key(req: &ServiceRequest) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .unwrap_or_else(|| {
            req.peer_addr()
                .map(|a| a.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn limits_the_expensive_unauthenticated_endpoints() {
        assert!(limit_for("POST", "/api/auth/login").is_some());
        assert!(limit_for("POST", "/api/auth/register").is_some());
        assert!(limit_for("POST", "/api/auth/password-reset").is_some());
        assert!(limit_for("POST", "/api/auth/refresh").is_some());
    }

    /// The token sits in the path, so this route cannot be matched by equality.
    #[test]
    fn matches_the_reset_completion_route_by_prefix() {
        assert!(limit_for("POST", "/api/auth/password-reset/some-token").is_some());
    }

    /// Registration is the most expensive: Argon2 plus an outbound email.
    #[test]
    fn registration_is_limited_harder_than_login() {
        let reg = limit_for("POST", "/api/auth/register").unwrap();
        let login = limit_for("POST", "/api/auth/login").unwrap();
        assert!(reg.limit < login.limit);
        assert!(reg.window_secs > login.window_secs);
    }

    #[test]
    fn leaves_authenticated_and_read_routes_alone() {
        assert!(limit_for("GET", "/api/blog").is_none());
        assert!(limit_for("POST", "/api/blog").is_none());
        assert!(limit_for("POST", "/api/cvs").is_none());
        assert!(limit_for("GET", "/api/auth/login").is_none());
    }

    /// peer_addr is the load balancer behind Cloud Run, so the forwarded header
    /// is what distinguishes callers.
    #[test]
    fn prefers_the_forwarded_client_address() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.7, 10.0.0.1"))
            .to_srv_request();
        assert_eq!(client_key(&req), "203.0.113.7");
    }

    #[test]
    fn ignores_a_blank_forwarded_header() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "   "))
            .to_srv_request();
        // No peer_addr in a synthetic request, so it falls through to the
        // placeholder rather than keying on an empty string.
        assert_eq!(client_key(&req), "unknown");
    }

    #[test]
    fn falls_back_when_no_forwarded_header_is_present() {
        let req = TestRequest::default().to_srv_request();
        assert_eq!(client_key(&req), "unknown");
    }
}
